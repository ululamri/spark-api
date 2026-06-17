use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{admin_auth, error::ApiError, state::AppState};

#[derive(Serialize)]
struct AdminEnvelope<T> {
    ok: bool,
    data: T,
    generated_at: DateTime<Utc>,
}

fn success<T>(data: T) -> Json<AdminEnvelope<T>> {
    Json(AdminEnvelope {
        ok: true,
        data,
        generated_at: Utc::now(),
    })
}

#[derive(Debug)]
enum AdminOpsError {
    NotConfigured,
    Unauthorized,
    BadRequest(String),
    NotFound(&'static str),
    Database(sqlx::Error),
    Internal,
}

#[derive(Serialize)]
struct AdminErrorEnvelope {
    ok: bool,
    error: AdminErrorBody,
}

#[derive(Serialize)]
struct AdminErrorBody {
    code: &'static str,
    message: String,
}

impl IntoResponse for AdminOpsError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::NotConfigured => (
                StatusCode::SERVICE_UNAVAILABLE,
                "admin_not_configured",
                "Admin access is not configured.".to_string(),
            ),
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "admin_unauthorized",
                "Admin access is not authorized.".to_string(),
            ),
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "admin_bad_request", message),
            Self::NotFound(entity) => (StatusCode::NOT_FOUND, "admin_not_found", entity.to_string()),
            Self::Database(error) => {
                tracing::error!(?error, "admin social ops database operation failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "admin_internal_error",
                    "The admin social operations request could not be completed.".to_string(),
                )
            }
            Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "admin_internal_error",
                "The admin social operations request could not be completed.".to_string(),
            ),
        };

        (
            status,
            Json(AdminErrorEnvelope {
                ok: false,
                error: AdminErrorBody { code, message },
            }),
        )
            .into_response()
    }
}

impl From<sqlx::Error> for AdminOpsError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

impl From<ApiError> for AdminOpsError {
    fn from(value: ApiError) -> Self {
        match value {
            ApiError::Unauthorized => Self::Unauthorized,
            ApiError::BadRequest(message) => Self::BadRequest(message),
            ApiError::ServiceUnavailable(_) => Self::NotConfigured,
            error => {
                tracing::error!(?error, "admin social ops authorization failed");
                Self::Internal
            }
        }
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/scope", get(scope))
        .route("/bulk-jobs", get(bulk_jobs))
        .route("/bulk-jobs/:job_id", get(bulk_job_detail))
}

#[derive(Serialize)]
struct ScopeData {
    module: &'static str,
    phase: &'static str,
    routes: Vec<&'static str>,
    data_source: &'static str,
    auth_model: &'static str,
}

async fn scope(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminEnvelope<ScopeData>>, AdminOpsError> {
    admin_auth::authorize_with_capability(&state, &headers, "moderation_read").await?;
    Ok(success(ScopeData {
        module: module_path!(),
        phase: "moderation-operations-history-observability",
        routes: vec![
            "GET /api/admin/social/ops/scope",
            "GET /api/admin/social/ops/bulk-jobs",
            "GET /api/admin/social/ops/bulk-jobs/:job_id",
        ],
        data_source: "database",
        auth_model: "legacy superadmin root plus delegated admin/moderator capabilities",
    }))
}

#[derive(Debug, Deserialize)]
struct BulkJobsQuery {
    limit: Option<i64>,
    offset: Option<i64>,
    status: Option<String>,
    target_type: Option<String>,
}

impl BulkJobsQuery {
    fn paging(&self) -> (i64, i64) {
        (
            self.limit.unwrap_or(25).clamp(1, 100),
            self.offset.unwrap_or(0).max(0),
        )
    }
}

#[derive(Debug, Serialize, FromRow)]
struct BulkJobRow {
    id: Uuid,
    actor_kind: String,
    actor_user_id: Option<Uuid>,
    target_type: String,
    action: String,
    reason: String,
    status: String,
    dry_run: bool,
    idempotency_key: Option<String>,
    total_count: i32,
    would_apply_count: i32,
    applied_count: i32,
    skipped_count: i32,
    failed_count: i32,
    payload: Value,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, FromRow)]
struct BulkJobItemRow {
    id: Uuid,
    bulk_job_id: Uuid,
    target_type: String,
    target_id: Uuid,
    action: String,
    status: String,
    action_id: Option<Uuid>,
    report_id: Option<Uuid>,
    message: String,
    metadata: Value,
    created_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct ListData<T> {
    items: Vec<T>,
    limit: i64,
    offset: i64,
    total: i64,
    data_source: &'static str,
}

#[derive(Serialize)]
struct BulkJobData {
    job: BulkJobRow,
    items: Vec<BulkJobItemRow>,
    data_source: &'static str,
}

async fn bulk_jobs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<BulkJobsQuery>,
) -> Result<Json<AdminEnvelope<ListData<BulkJobRow>>>, AdminOpsError> {
    admin_auth::authorize_with_capability(&state, &headers, "moderation_read").await?;
    let (limit, offset) = query.paging();
    let status = query.status.as_deref().map(normalize_job_status).transpose()?;
    let target_type = query
        .target_type
        .as_deref()
        .map(normalize_target_type)
        .transpose()?;

    let total = sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)
        from social_moderation_bulk_jobs
        where ($1::text is null or status = $1)
          and ($2::text is null or target_type = $2)
        "#,
    )
    .bind(status.as_deref())
    .bind(target_type.as_deref())
    .fetch_one(&state.db)
    .await?;

    let items = sqlx::query_as::<_, BulkJobRow>(
        r#"
        select id, actor_kind, actor_user_id, target_type, action, reason,
               status, dry_run, idempotency_key, total_count,
               would_apply_count, applied_count, skipped_count, failed_count,
               payload, created_at, completed_at
        from social_moderation_bulk_jobs
        where ($1::text is null or status = $1)
          and ($2::text is null or target_type = $2)
        order by created_at desc, id desc
        limit $3 offset $4
        "#,
    )
    .bind(status.as_deref())
    .bind(target_type.as_deref())
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    Ok(success(ListData {
        items,
        limit,
        offset,
        total,
        data_source: "database",
    }))
}

async fn bulk_job_detail(
    Path(job_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminEnvelope<BulkJobData>>, AdminOpsError> {
    admin_auth::authorize_with_capability(&state, &headers, "moderation_read").await?;
    Ok(success(fetch_bulk_job_data(&state, job_id).await?))
}

async fn fetch_bulk_job_data(state: &AppState, job_id: Uuid) -> Result<BulkJobData, AdminOpsError> {
    let job = sqlx::query_as::<_, BulkJobRow>(
        r#"
        select id, actor_kind, actor_user_id, target_type, action, reason,
               status, dry_run, idempotency_key, total_count,
               would_apply_count, applied_count, skipped_count, failed_count,
               payload, created_at, completed_at
        from social_moderation_bulk_jobs
        where id = $1
        "#,
    )
    .bind(job_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AdminOpsError::NotFound("Bulk moderation job was not found."))?;

    let items = sqlx::query_as::<_, BulkJobItemRow>(
        r#"
        select id, bulk_job_id, target_type, target_id, action, status,
               action_id, report_id, message, metadata, created_at
        from social_moderation_bulk_job_items
        where bulk_job_id = $1
        order by created_at asc, id asc
        "#,
    )
    .bind(job_id)
    .fetch_all(&state.db)
    .await?;

    Ok(BulkJobData {
        job,
        items,
        data_source: "database",
    })
}

fn normalize_job_status(input: &str) -> Result<String, AdminOpsError> {
    match input.trim() {
        "running" | "dry_run" | "completed" | "partial_failed" | "failed" => {
            Ok(input.trim().to_string())
        }
        _ => Err(AdminOpsError::BadRequest(
            "status must be running, dry_run, completed, partial_failed, or failed".to_string(),
        )),
    }
}

fn normalize_target_type(input: &str) -> Result<String, AdminOpsError> {
    match input.trim() {
        "post" | "comment" | "report" => Ok(input.trim().to_string()),
        _ => Err(AdminOpsError::BadRequest(
            "target_type must be post, comment, or report".to_string(),
        )),
    }
}
