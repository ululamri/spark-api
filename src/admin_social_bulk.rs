use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use std::collections::HashSet;
use uuid::Uuid;

use crate::{admin_auth, error::ApiError, state::AppState};

const MAX_BULK_TARGETS: usize = 100;

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
enum AdminBulkError {
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

impl IntoResponse for AdminBulkError {
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
                tracing::error!(?error, "admin social bulk database operation failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "admin_internal_error",
                    "The admin social bulk request could not be completed.".to_string(),
                )
            }
            Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "admin_internal_error",
                "The admin social bulk request could not be completed.".to_string(),
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

impl From<sqlx::Error> for AdminBulkError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

impl From<ApiError> for AdminBulkError {
    fn from(value: ApiError) -> Self {
        match value {
            ApiError::Unauthorized => Self::Unauthorized,
            ApiError::BadRequest(message) => Self::BadRequest(message),
            ApiError::ServiceUnavailable(_) => Self::NotConfigured,
            error => {
                tracing::error!(?error, "admin social bulk authorization failed");
                Self::Internal
            }
        }
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/scope", get(scope))
        .route("/moderation-actions", post(create_bulk_moderation_action))
        .route("/jobs/:job_id", get(bulk_job_detail))
}

#[derive(Serialize)]
struct ScopeData {
    module: &'static str,
    phase: &'static str,
    routes: Vec<&'static str>,
    actions: Vec<&'static str>,
    safeguards: Vec<&'static str>,
    data_source: &'static str,
    auth_model: &'static str,
}

async fn scope(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminEnvelope<ScopeData>>, AdminBulkError> {
    admin_auth::authorize_with_capability(&state, &headers, "moderation_read").await?;

    Ok(success(ScopeData {
        module: module_path!(),
        phase: "advanced-social-moderation-bulk-action-engine",
        routes: vec![
            "GET /api/admin/social/bulk/scope",
            "POST /api/admin/social/bulk/moderation-actions",
            "GET /api/admin/social/bulk/jobs/:job_id",
        ],
        actions: vec![
            "hide",
            "remove",
            "restore",
            "dismiss_report",
            "mark_reviewed",
        ],
        safeguards: vec![
            "requires moderation_bulk and the specific action capability",
            "dry_run mode validates targets without mutating content",
            "idempotency_key prevents duplicate bulk execution",
            "per-item result rows are persisted",
            "per-item and per-job admin audit events are written",
        ],
        data_source: "database",
        auth_model: "legacy superadmin root plus delegated admin/moderator capabilities",
    }))
}

#[derive(Debug, Deserialize)]
struct BulkModerationRequest {
    target_type: String,
    action: String,
    reason: Option<String>,
    dry_run: Option<bool>,
    idempotency_key: Option<String>,
    target_ids: Option<Vec<Uuid>>,
    targets: Option<Vec<BulkModerationTargetInput>>,
    payload: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct BulkModerationTargetInput {
    target_id: Uuid,
    report_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
struct BulkTarget {
    target_id: Uuid,
    report_id: Option<Uuid>,
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

#[derive(Debug, Serialize, FromRow)]
struct ModerationActionRow {
    id: Uuid,
    moderator_user_id: Option<Uuid>,
    target_type: String,
    target_id: Uuid,
    action: String,
    reason: String,
    payload: Value,
    created_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct BulkJobData {
    job: BulkJobRow,
    items: Vec<BulkJobItemRow>,
    data_source: &'static str,
}

#[derive(Default)]
struct BulkCounters {
    would_apply: i32,
    applied: i32,
    skipped: i32,
    failed: i32,
}

async fn create_bulk_moderation_action(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<BulkModerationRequest>,
) -> Result<(StatusCode, Json<AdminEnvelope<BulkJobData>>), AdminBulkError> {
    let target_type = normalize_moderation_target_type(&payload.target_type)?;
    let action = normalize_moderation_action(&payload.action)?;
    validate_action_target_pair(&target_type, &action)?;

    let actor =
        admin_auth::authorize_with_capability(&state, &headers, "moderation_bulk").await?;
    ensure_actor_capability(&actor, capability_for_moderation_action(&action))?;

    let reason = clean_reason(payload.reason.as_deref())?;
    let dry_run = payload.dry_run.unwrap_or(false);
    let idempotency_key = clean_idempotency_key(payload.idempotency_key.as_deref())?;
    let request_payload = clean_payload(payload.payload.unwrap_or_else(|| json!({})))?;
    let targets = clean_bulk_targets(payload.target_ids, payload.targets)?;

    if let Some(existing_key) = idempotency_key.as_deref() {
        if let Some(existing_job_id) = find_existing_job_by_idempotency_key(&state, existing_key).await? {
            let data = fetch_bulk_job_data(&state, existing_job_id).await?;
            return Ok((StatusCode::OK, success(data)));
        }
    }

    let job_id = Uuid::new_v4();
    let job = insert_bulk_job(
        &state,
        job_id,
        &actor,
        &target_type,
        &action,
        &reason,
        dry_run,
        idempotency_key.as_deref(),
        targets.len() as i32,
        &request_payload,
    )
    .await?;

    let mut counters = BulkCounters::default();

    for target in targets {
        let item = process_bulk_target(
            &state,
            &actor,
            &job,
            &target_type,
            &action,
            &reason,
            dry_run,
            &request_payload,
            target,
        )
        .await?;

        match item.status.as_str() {
            "would_apply" => counters.would_apply += 1,
            "applied" => counters.applied += 1,
            "skipped" => counters.skipped += 1,
            _ => counters.failed += 1,
        }
    }

    let final_status = final_job_status(dry_run, &counters);
    update_bulk_job_counts(&state, job_id, final_status, &counters).await?;

    let data = fetch_bulk_job_data(&state, job_id).await?;

    admin_auth::audit(
        &state,
        &actor,
        "social_bulk_moderation_job",
        "social_moderation_bulk_job",
        None,
        Some(job_id),
        &actor.capabilities,
        "Bulk social moderation job completed.",
        json!({
            "job_id": job_id,
            "target_type": target_type,
            "action": action,
            "dry_run": dry_run,
            "status": final_status,
            "total_count": data.job.total_count,
            "would_apply_count": data.job.would_apply_count,
            "applied_count": data.job.applied_count,
            "skipped_count": data.job.skipped_count,
            "failed_count": data.job.failed_count,
            "idempotency_key": data.job.idempotency_key,
        }),
    )
    .await?;

    Ok((StatusCode::CREATED, success(data)))
}

async fn bulk_job_detail(
    Path(job_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminEnvelope<BulkJobData>>, AdminBulkError> {
    admin_auth::authorize_with_capability(&state, &headers, "moderation_read").await?;
    Ok(success(fetch_bulk_job_data(&state, job_id).await?))
}

async fn insert_bulk_job(
    state: &AppState,
    job_id: Uuid,
    actor: &admin_auth::AdminContext,
    target_type: &str,
    action: &str,
    reason: &str,
    dry_run: bool,
    idempotency_key: Option<&str>,
    total_count: i32,
    request_payload: &Value,
) -> Result<BulkJobRow, AdminBulkError> {
    Ok(sqlx::query_as::<_, BulkJobRow>(
        r#"
        insert into social_moderation_bulk_jobs (
          id, actor_kind, actor_user_id, target_type, action, reason,
          status, dry_run, idempotency_key, total_count, payload
        ) values ($1, $2, $3, $4, $5, $6, 'running', $7, $8, $9, $10)
        returning id, actor_kind, actor_user_id, target_type, action, reason,
                  status, dry_run, idempotency_key, total_count,
                  would_apply_count, applied_count, skipped_count, failed_count,
                  payload, created_at, completed_at
        "#,
    )
    .bind(job_id)
    .bind(&actor.actor_kind)
    .bind(actor.actor_user_id)
    .bind(target_type)
    .bind(action)
    .bind(reason)
    .bind(dry_run)
    .bind(idempotency_key)
    .bind(total_count)
    .bind(json!({
        "request": request_payload,
        "actor_kind": &actor.actor_kind,
        "actor_role": &actor.role,
    }))
    .fetch_one(&state.db)
    .await?)
}

async fn process_bulk_target(
    state: &AppState,
    actor: &admin_auth::AdminContext,
    job: &BulkJobRow,
    target_type: &str,
    action: &str,
    reason: &str,
    dry_run: bool,
    request_payload: &Value,
    target: BulkTarget,
) -> Result<BulkJobItemRow, AdminBulkError> {
    let item_id = Uuid::new_v4();
    let current_status = fetch_target_status(state, target_type, target.target_id).await?;
    let (status, action_id, message, metadata) = match current_status.as_deref() {
        None => (
            "failed".to_string(),
            None,
            "target was not found".to_string(),
            json!({ "exists": false }),
        ),
        Some("deleted") => (
            "failed".to_string(),
            None,
            "deleted content cannot be changed by bulk moderation".to_string(),
            json!({ "exists": true, "current_status": "deleted" }),
        ),
        Some(current) if Some(current) == desired_status_for_action(action) => (
            "skipped".to_string(),
            None,
            "target already has the desired status".to_string(),
            json!({ "exists": true, "current_status": current }),
        ),
        Some(current) if dry_run => (
            "would_apply".to_string(),
            None,
            "dry_run: target exists and action would be applied".to_string(),
            json!({ "exists": true, "current_status": current }),
        ),
        Some(current) => {
            let action_row = insert_moderation_action(
                state,
                actor,
                job.id,
                item_id,
                target_type,
                target.target_id,
                action,
                reason,
                request_payload,
            )
            .await?;

            apply_moderation_action(state, &action_row, target.report_id, actor.actor_user_id)
                .await?;

            (
                "applied".to_string(),
                Some(action_row.id),
                "moderation action applied".to_string(),
                json!({ "exists": true, "previous_status": current }),
            )
        }
    };

    let item = insert_bulk_job_item(
        state,
        item_id,
        job.id,
        target_type,
        target.target_id,
        action,
        &status,
        action_id,
        target.report_id,
        &message,
        metadata,
    )
    .await?;

    admin_auth::audit(
        state,
        actor,
        "social_bulk_moderation_item",
        target_type,
        None,
        Some(target.target_id),
        &actor.capabilities,
        "Bulk social moderation item processed.",
        json!({
            "bulk_job_id": job.id,
            "bulk_item_id": item.id,
            "target_type": target_type,
            "target_id": target.target_id,
            "action": action,
            "status": item.status,
            "action_id": item.action_id,
            "report_id": item.report_id,
            "dry_run": dry_run,
            "message": item.message,
        }),
    )
    .await?;

    Ok(item)
}

async fn insert_moderation_action(
    state: &AppState,
    actor: &admin_auth::AdminContext,
    bulk_job_id: Uuid,
    bulk_item_id: Uuid,
    target_type: &str,
    target_id: Uuid,
    action: &str,
    reason: &str,
    request_payload: &Value,
) -> Result<ModerationActionRow, AdminBulkError> {
    Ok(sqlx::query_as::<_, ModerationActionRow>(
        r#"
        insert into social_moderation_actions (
          id, moderator_user_id, target_type, target_id, action, reason, payload
        )
        values ($1, $2, $3, $4, $5, $6, $7)
        returning id, moderator_user_id, target_type, target_id, action, reason, payload, created_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(actor.actor_user_id)
    .bind(target_type)
    .bind(target_id)
    .bind(action)
    .bind(reason)
    .bind(json!({
        "request": request_payload,
        "actor_kind": &actor.actor_kind,
        "actor_role": &actor.role,
        "bulk_job_id": bulk_job_id,
        "bulk_item_id": bulk_item_id,
    }))
    .fetch_one(&state.db)
    .await?)
}

async fn insert_bulk_job_item(
    state: &AppState,
    item_id: Uuid,
    bulk_job_id: Uuid,
    target_type: &str,
    target_id: Uuid,
    action: &str,
    status: &str,
    action_id: Option<Uuid>,
    report_id: Option<Uuid>,
    message: &str,
    metadata: Value,
) -> Result<BulkJobItemRow, AdminBulkError> {
    Ok(sqlx::query_as::<_, BulkJobItemRow>(
        r#"
        insert into social_moderation_bulk_job_items (
          id, bulk_job_id, target_type, target_id, action, status,
          action_id, report_id, message, metadata
        ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        returning id, bulk_job_id, target_type, target_id, action, status,
                  action_id, report_id, message, metadata, created_at
        "#,
    )
    .bind(item_id)
    .bind(bulk_job_id)
    .bind(target_type)
    .bind(target_id)
    .bind(action)
    .bind(status)
    .bind(action_id)
    .bind(report_id)
    .bind(message)
    .bind(metadata)
    .fetch_one(&state.db)
    .await?)
}

async fn update_bulk_job_counts(
    state: &AppState,
    job_id: Uuid,
    status: &str,
    counters: &BulkCounters,
) -> Result<(), AdminBulkError> {
    sqlx::query(
        r#"
        update social_moderation_bulk_jobs
        set status = $2,
            would_apply_count = $3,
            applied_count = $4,
            skipped_count = $5,
            failed_count = $6,
            completed_at = now()
        where id = $1
        "#,
    )
    .bind(job_id)
    .bind(status)
    .bind(counters.would_apply)
    .bind(counters.applied)
    .bind(counters.skipped)
    .bind(counters.failed)
    .execute(&state.db)
    .await?;

    Ok(())
}

async fn fetch_bulk_job_data(
    state: &AppState,
    job_id: Uuid,
) -> Result<BulkJobData, AdminBulkError> {
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
    .ok_or(AdminBulkError::NotFound("Bulk moderation job was not found."))?;

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

async fn find_existing_job_by_idempotency_key(
    state: &AppState,
    idempotency_key: &str,
) -> Result<Option<Uuid>, AdminBulkError> {
    Ok(sqlx::query_scalar::<_, Uuid>(
        "select id from social_moderation_bulk_jobs where idempotency_key = $1 limit 1",
    )
    .bind(idempotency_key)
    .fetch_optional(&state.db)
    .await?)
}

async fn fetch_target_status(
    state: &AppState,
    target_type: &str,
    target_id: Uuid,
) -> Result<Option<String>, AdminBulkError> {
    let status = match target_type {
        "post" => {
            sqlx::query_scalar::<_, String>("select status from social_posts where id = $1")
                .bind(target_id)
                .fetch_optional(&state.db)
                .await?
        }
        "comment" => {
            sqlx::query_scalar::<_, String>("select status from social_comments where id = $1")
                .bind(target_id)
                .fetch_optional(&state.db)
                .await?
        }
        "report" => {
            sqlx::query_scalar::<_, String>("select status from social_reports where id = $1")
                .bind(target_id)
                .fetch_optional(&state.db)
                .await?
        }
        _ => None,
    };

    Ok(status)
}

async fn apply_moderation_action(
    state: &AppState,
    action_row: &ModerationActionRow,
    report_id: Option<Uuid>,
    actor_user_id: Option<Uuid>,
) -> Result<(), AdminBulkError> {
    match (action_row.target_type.as_str(), action_row.action.as_str()) {
        ("post", "hide") => {
            update_post_status(state, action_row.target_id, "hidden", action_row.id).await?
        }
        ("post", "remove") => {
            update_post_status(state, action_row.target_id, "removed", action_row.id).await?
        }
        ("post", "restore") => {
            update_post_status(state, action_row.target_id, "published", action_row.id).await?
        }
        ("comment", "hide") => {
            update_comment_status(state, action_row.target_id, "hidden", action_row.id).await?
        }
        ("comment", "remove") => {
            update_comment_status(state, action_row.target_id, "removed", action_row.id).await?
        }
        ("comment", "restore") => {
            update_comment_status(state, action_row.target_id, "published", action_row.id).await?
        }
        ("report", "dismiss_report") => {
            update_report_status(state, action_row.target_id, "dismissed", action_row.id, actor_user_id)
                .await?
        }
        ("report", "mark_reviewed") => {
            update_report_status(state, action_row.target_id, "reviewed", action_row.id, actor_user_id)
                .await?
        }
        _ => {
            return Err(AdminBulkError::BadRequest(
                "unsupported moderation action".to_string(),
            ))
        }
    }

    if let Some(report_id) = report_id {
        update_report_status(state, report_id, "actioned", action_row.id, actor_user_id).await?;
    }

    Ok(())
}

async fn update_post_status(
    state: &AppState,
    post_id: Uuid,
    status: &str,
    action_id: Uuid,
) -> Result<(), AdminBulkError> {
    let result = sqlx::query(
        r#"
        update social_posts
        set status = $2,
            hidden_at = case when $2 = 'hidden' then now() when $2 = 'published' then null else hidden_at end,
            removed_at = case when $2 = 'removed' then now() when $2 = 'published' then null else removed_at end,
            updated_at = now(),
            metadata = jsonb_set(metadata, '{last_moderation_action_id}', to_jsonb($3::text), true)
        where id = $1
        "#,
    )
    .bind(post_id)
    .bind(status)
    .bind(action_id.to_string())
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        Err(AdminBulkError::NotFound("Social post was not found."))
    } else {
        Ok(())
    }
}

async fn update_comment_status(
    state: &AppState,
    comment_id: Uuid,
    status: &str,
    action_id: Uuid,
) -> Result<(), AdminBulkError> {
    let result = sqlx::query(
        r#"
        update social_comments
        set status = $2,
            hidden_at = case when $2 = 'hidden' then now() when $2 = 'published' then null else hidden_at end,
            removed_at = case when $2 = 'removed' then now() when $2 = 'published' then null else removed_at end,
            updated_at = now(),
            metadata = jsonb_set(metadata, '{last_moderation_action_id}', to_jsonb($3::text), true)
        where id = $1
        "#,
    )
    .bind(comment_id)
    .bind(status)
    .bind(action_id.to_string())
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        Err(AdminBulkError::NotFound("Social comment was not found."))
    } else {
        Ok(())
    }
}

async fn update_report_status(
    state: &AppState,
    report_id: Uuid,
    status: &str,
    action_id: Uuid,
    actor_user_id: Option<Uuid>,
) -> Result<(), AdminBulkError> {
    let result = sqlx::query(
        r#"
        update social_reports
        set status = $2,
            action_id = $3,
            reviewed_by_user_id = coalesce(reviewed_by_user_id, $4),
            reviewed_at = coalesce(reviewed_at, now()),
            updated_at = now()
        where id = $1
        "#,
    )
    .bind(report_id)
    .bind(status)
    .bind(action_id)
    .bind(actor_user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        Err(AdminBulkError::NotFound("Social report was not found."))
    } else {
        Ok(())
    }
}

fn clean_bulk_targets(
    target_ids: Option<Vec<Uuid>>,
    targets: Option<Vec<BulkModerationTargetInput>>,
) -> Result<Vec<BulkTarget>, AdminBulkError> {
    let raw_targets = match targets {
        Some(items) if !items.is_empty() => items
            .into_iter()
            .map(|item| BulkTarget {
                target_id: item.target_id,
                report_id: item.report_id,
            })
            .collect::<Vec<_>>(),
        _ => target_ids
            .unwrap_or_default()
            .into_iter()
            .map(|target_id| BulkTarget {
                target_id,
                report_id: None,
            })
            .collect::<Vec<_>>(),
    };

    if raw_targets.is_empty() {
        return Err(AdminBulkError::BadRequest(
            "target_ids or targets must contain at least one item".to_string(),
        ));
    }

    if raw_targets.len() > MAX_BULK_TARGETS {
        return Err(AdminBulkError::BadRequest(format!(
            "bulk moderation is limited to {MAX_BULK_TARGETS} targets per request"
        )));
    }

    let mut seen = HashSet::new();
    let mut output = Vec::with_capacity(raw_targets.len());

    for target in raw_targets {
        if seen.insert(target.target_id) {
            output.push(target);
        }
    }

    Ok(output)
}

fn ensure_actor_capability(
    actor: &admin_auth::AdminContext,
    capability: &str,
) -> Result<(), AdminBulkError> {
    if actor.capabilities.iter().any(|item| item == capability) {
        Ok(())
    } else {
        Err(AdminBulkError::Unauthorized)
    }
}

fn clean_reason(input: Option<&str>) -> Result<String, AdminBulkError> {
    let value = input.unwrap_or("").trim();
    if value.chars().count() > 1000 {
        return Err(AdminBulkError::BadRequest(
            "reason is too long".to_string(),
        ));
    }
    if value.chars().any(char::is_control) {
        return Err(AdminBulkError::BadRequest(
            "reason cannot contain control characters".to_string(),
        ));
    }
    Ok(value.to_string())
}

fn clean_idempotency_key(input: Option<&str>) -> Result<Option<String>, AdminBulkError> {
    let Some(raw) = input else {
        return Ok(None);
    };
    let value = raw.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > 128 {
        return Err(AdminBulkError::BadRequest(
            "idempotency_key is too long".to_string(),
        ));
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '.'))
    {
        return Err(AdminBulkError::BadRequest(
            "idempotency_key may only contain ASCII letters, numbers, dash, underscore, colon, or dot".to_string(),
        ));
    }
    Ok(Some(value.to_string()))
}

fn clean_payload(value: Value) -> Result<Value, AdminBulkError> {
    if value.to_string().chars().count() > 20_000 {
        return Err(AdminBulkError::BadRequest(
            "payload is too large".to_string(),
        ));
    }
    Ok(value)
}

fn normalize_moderation_target_type(input: &str) -> Result<String, AdminBulkError> {
    let value = input.trim().to_ascii_lowercase();
    match value.as_str() {
        "post" | "comment" | "report" => Ok(value),
        _ => Err(AdminBulkError::BadRequest(
            "target_type must be post, comment, or report".to_string(),
        )),
    }
}

fn normalize_moderation_action(input: &str) -> Result<String, AdminBulkError> {
    let value = input.trim().to_ascii_lowercase();
    match value.as_str() {
        "hide" | "remove" | "restore" | "dismiss_report" | "mark_reviewed" => Ok(value),
        _ => Err(AdminBulkError::BadRequest(
            "action must be hide, remove, restore, dismiss_report, or mark_reviewed".to_string(),
        )),
    }
}

fn capability_for_moderation_action(action: &str) -> &'static str {
    match action {
        "restore" => "moderation_restore",
        "dismiss_report" | "mark_reviewed" => "reports_manage",
        _ => "moderation_action",
    }
}

fn validate_action_target_pair(target_type: &str, action: &str) -> Result<(), AdminBulkError> {
    match (target_type, action) {
        ("post" | "comment", "hide" | "remove" | "restore") => Ok(()),
        ("report", "dismiss_report" | "mark_reviewed") => Ok(()),
        _ => Err(AdminBulkError::BadRequest(
            "action is not valid for the given target_type".to_string(),
        )),
    }
}

fn desired_status_for_action(action: &str) -> Option<&'static str> {
    match action {
        "hide" => Some("hidden"),
        "remove" => Some("removed"),
        "restore" => Some("published"),
        "dismiss_report" => Some("dismissed"),
        "mark_reviewed" => Some("reviewed"),
        _ => None,
    }
}

fn final_job_status(dry_run: bool, counters: &BulkCounters) -> &'static str {
    if dry_run {
        return "dry_run";
    }

    if counters.failed > 0 && counters.applied == 0 && counters.skipped == 0 {
        "failed"
    } else if counters.failed > 0 {
        "partial_failed"
    } else {
        "completed"
    }
}
