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
enum AdminAuditError {
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

impl IntoResponse for AdminAuditError {
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
                tracing::error!(?error, "admin audit database operation failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "admin_internal_error",
                    "The admin audit request could not be completed.".to_string(),
                )
            }
            Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "admin_internal_error",
                "The admin audit request could not be completed.".to_string(),
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

impl From<sqlx::Error> for AdminAuditError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

impl From<ApiError> for AdminAuditError {
    fn from(value: ApiError) -> Self {
        match value {
            ApiError::Unauthorized => Self::Unauthorized,
            ApiError::BadRequest(message) => Self::BadRequest(message),
            ApiError::ServiceUnavailable(_) => Self::NotConfigured,
            error => {
                tracing::error!(?error, "admin audit authorization failed");
                Self::Internal
            }
        }
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/scope", get(scope))
        .route("/events", get(events))
        .route("/events/:event_id", get(event_detail))
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
) -> Result<Json<AdminEnvelope<ScopeData>>, AdminAuditError> {
    admin_auth::authorize_with_capability(&state, &headers, "audit_read").await?;
    Ok(success(ScopeData {
        module: module_path!(),
        phase: "admin-audit-log-viewer",
        routes: vec![
            "GET /api/admin/audit/scope",
            "GET /api/admin/audit/events",
            "GET /api/admin/audit/events/:event_id",
        ],
        data_source: "database",
        auth_model: "legacy superadmin root plus delegated admin audit_read capability",
    }))
}

#[derive(Debug, Deserialize)]
struct AuditQuery {
    limit: Option<i64>,
    offset: Option<i64>,
    actor_kind: Option<String>,
    action: Option<String>,
    target_type: Option<String>,
}

impl AuditQuery {
    fn paging(&self) -> (i64, i64) {
        (
            self.limit.unwrap_or(50).clamp(1, 100),
            self.offset.unwrap_or(0).max(0),
        )
    }
}

#[derive(Debug, Serialize, FromRow)]
struct AuditEventRow {
    id: Uuid,
    actor_kind: String,
    actor_user_id: Option<Uuid>,
    action: String,
    target_type: String,
    target_user_id: Option<Uuid>,
    target_id: Option<Uuid>,
    capabilities: Vec<String>,
    summary: String,
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

async fn events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AuditQuery>,
) -> Result<Json<AdminEnvelope<ListData<AuditEventRow>>>, AdminAuditError> {
    admin_auth::authorize_with_capability(&state, &headers, "audit_read").await?;
    let (limit, offset) = query.paging();
    let actor_kind = clean_filter(query.actor_kind.as_deref(), "actor_kind")?;
    let action = clean_filter(query.action.as_deref(), "action")?;
    let target_type = clean_filter(query.target_type.as_deref(), "target_type")?;

    let total = sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)
        from admin_audit_events
        where ($1::text is null or actor_kind = $1)
          and ($2::text is null or action = $2)
          and ($3::text is null or target_type = $3)
        "#,
    )
    .bind(actor_kind.as_deref())
    .bind(action.as_deref())
    .bind(target_type.as_deref())
    .fetch_one(&state.db)
    .await?;

    let items = sqlx::query_as::<_, AuditEventRow>(
        r#"
        select id, actor_kind, actor_user_id, action, target_type,
               target_user_id, target_id, capabilities, summary, metadata, created_at
        from admin_audit_events
        where ($1::text is null or actor_kind = $1)
          and ($2::text is null or action = $2)
          and ($3::text is null or target_type = $3)
        order by created_at desc, id desc
        limit $4 offset $5
        "#,
    )
    .bind(actor_kind.as_deref())
    .bind(action.as_deref())
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

async fn event_detail(
    Path(event_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminEnvelope<AuditEventRow>>, AdminAuditError> {
    admin_auth::authorize_with_capability(&state, &headers, "audit_read").await?;
    let event = sqlx::query_as::<_, AuditEventRow>(
        r#"
        select id, actor_kind, actor_user_id, action, target_type,
               target_user_id, target_id, capabilities, summary, metadata, created_at
        from admin_audit_events
        where id = $1
        "#,
    )
    .bind(event_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AdminAuditError::NotFound("Audit event was not found."))?;
    Ok(success(event))
}

fn clean_filter(input: Option<&str>, name: &str) -> Result<Option<String>, AdminAuditError> {
    let Some(value) = input.map(str::trim).filter(|value| !value.is_empty() && *value != "all") else {
        return Ok(None);
    };
    if value.chars().count() > 80 {
        return Err(AdminAuditError::BadRequest(format!("{name} filter is too long")));
    }
    if value.chars().any(char::is_control) {
        return Err(AdminAuditError::BadRequest(format!(
            "{name} filter cannot contain control characters"
        )));
    }
    Ok(Some(value.to_string()))
}
