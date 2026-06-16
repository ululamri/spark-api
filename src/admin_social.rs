use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use uuid::Uuid;

use crate::state::AppState;

const ADMIN_HEADER: &str = "x-karyra-admin-token";

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
enum AdminSocialError {
    NotConfigured,
    Unauthorized,
    BadRequest(String),
    NotFound(&'static str),
    Database(sqlx::Error),
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

impl IntoResponse for AdminSocialError {
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
            Self::NotFound(entity) => {
                (StatusCode::NOT_FOUND, "admin_not_found", entity.to_string())
            }
            Self::Database(error) => {
                tracing::error!(?error, "admin social database operation failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "admin_internal_error",
                    "The admin social request could not be completed.".to_string(),
                )
            }
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

impl From<sqlx::Error> for AdminSocialError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/scope", get(scope))
        .route("/reports", get(reports))
        .route("/posts", get(posts))
        .route("/comments", get(comments))
        .route("/moderation-actions", post(create_moderation_action))
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<(), AdminSocialError> {
    let configured = state
        .config
        .admin_token
        .as_deref()
        .ok_or(AdminSocialError::NotConfigured)?;
    let supplied = headers
        .get(ADMIN_HEADER)
        .and_then(|value| value.to_str().ok())
        .ok_or(AdminSocialError::Unauthorized)?;

    if Sha256::digest(configured.as_bytes()) == Sha256::digest(supplied.as_bytes()) {
        Ok(())
    } else {
        Err(AdminSocialError::Unauthorized)
    }
}

#[derive(Serialize)]
struct ScopeData {
    module: &'static str,
    phase: &'static str,
    routes: Vec<&'static str>,
    actions: Vec<&'static str>,
    data_source: &'static str,
}

async fn scope(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminEnvelope<ScopeData>>, AdminSocialError> {
    authorize(&state, &headers)?;
    Ok(success(ScopeData {
        module: module_path!(),
        phase: "public-social-admin-moderation",
        routes: vec![
            "GET /api/admin/social/reports",
            "GET /api/admin/social/posts",
            "GET /api/admin/social/comments",
            "POST /api/admin/social/moderation-actions",
        ],
        actions: vec![
            "hide",
            "remove",
            "restore",
            "dismiss_report",
            "mark_reviewed",
        ],
        data_source: "database",
    }))
}

#[derive(Debug, Deserialize)]
struct AdminSocialQuery {
    limit: Option<i64>,
    offset: Option<i64>,
    status: Option<String>,
    target_type: Option<String>,
}

impl AdminSocialQuery {
    fn paging(&self) -> (i64, i64) {
        (
            self.limit.unwrap_or(50).clamp(1, 100),
            self.offset.unwrap_or(0).max(0),
        )
    }
}

#[derive(Serialize)]
struct ListData<T> {
    items: Vec<T>,
    limit: i64,
    offset: i64,
    total: i64,
    data_source: &'static str,
}

#[derive(Serialize, FromRow)]
struct SocialReportItem {
    id: Uuid,
    reporter_user_id: Uuid,
    reporter_display_name: String,
    target_type: String,
    target_id: Uuid,
    reason: String,
    details: String,
    status: String,
    reviewed_by_user_id: Option<Uuid>,
    reviewed_at: Option<DateTime<Utc>>,
    action_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

async fn reports(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminSocialQuery>,
) -> Result<Json<AdminEnvelope<ListData<SocialReportItem>>>, AdminSocialError> {
    authorize(&state, &headers)?;
    let (limit, offset) = query.paging();
    let status = normalize_optional_status(query.status.as_deref())?;
    let target_type = normalize_optional_target_type(query.target_type.as_deref())?;

    let total = sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)
        from social_reports sr
        where ($1::text is null or sr.status = $1)
          and ($2::text is null or sr.target_type = $2)
        "#,
    )
    .bind(status.as_deref())
    .bind(target_type.as_deref())
    .fetch_one(&state.db)
    .await?;

    let rows = sqlx::query_as::<_, SocialReportItem>(
        r#"
        select sr.id,
               sr.reporter_user_id,
               coalesce(nullif(p.display_name, ''), 'Pengguna Spark') as reporter_display_name,
               sr.target_type,
               sr.target_id,
               sr.reason,
               sr.details,
               sr.status,
               sr.reviewed_by_user_id,
               sr.reviewed_at,
               sr.action_id,
               sr.created_at,
               sr.updated_at
        from social_reports sr
        left join profiles p on p.user_id = sr.reporter_user_id
        where ($1::text is null or sr.status = $1)
          and ($2::text is null or sr.target_type = $2)
        order by
          case when sr.status = 'pending' then 0 else 1 end,
          sr.created_at asc,
          sr.id asc
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
        items: rows,
        limit,
        offset,
        total,
        data_source: "database",
    }))
}

#[derive(Serialize, FromRow)]
struct SocialPostAdminItem {
    id: Uuid,
    author_user_id: Uuid,
    author_display_name: String,
    kind: String,
    body: String,
    visibility: String,
    status: String,
    comments_count: i64,
    reactions: Value,
    reports_count: i64,
    published_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

async fn posts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminSocialQuery>,
) -> Result<Json<AdminEnvelope<ListData<SocialPostAdminItem>>>, AdminSocialError> {
    authorize(&state, &headers)?;
    let (limit, offset) = query.paging();
    let status = normalize_optional_content_status(query.status.as_deref())?;

    let total = sqlx::query_scalar::<_, i64>(
        "select count(*) from social_posts where ($1::text is null or status = $1)",
    )
    .bind(status.as_deref())
    .fetch_one(&state.db)
    .await?;

    let rows = sqlx::query_as::<_, SocialPostAdminItem>(
        r#"
        select sp.id,
               sp.author_user_id,
               coalesce(nullif(p.display_name, ''), 'Pengguna Spark') as author_display_name,
               sp.kind,
               sp.body,
               sp.visibility,
               sp.status,
               (select count(*) from social_comments sc where sc.post_id = sp.id) as comments_count,
               coalesce((
                 select jsonb_object_agg(kind, total)
                 from (
                   select sr.kind, count(*)::bigint as total
                   from social_reactions sr
                   where sr.post_id = sp.id
                   group by sr.kind
                 ) reaction_counts
               ), '{}'::jsonb) as reactions,
               (select count(*) from social_reports r where r.target_type = 'post' and r.target_id = sp.id) as reports_count,
               sp.published_at,
               sp.created_at,
               sp.updated_at
        from social_posts sp
        left join profiles p on p.user_id = sp.author_user_id
        where ($1::text is null or sp.status = $1)
        order by sp.created_at desc, sp.id desc
        limit $2 offset $3
        "#,
    )
    .bind(status.as_deref())
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    Ok(success(ListData {
        items: rows,
        limit,
        offset,
        total,
        data_source: "database",
    }))
}

#[derive(Serialize, FromRow)]
struct SocialCommentAdminItem {
    id: Uuid,
    post_id: Uuid,
    author_user_id: Uuid,
    author_display_name: String,
    parent_comment_id: Option<Uuid>,
    body: String,
    status: String,
    reactions: Value,
    reports_count: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

async fn comments(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminSocialQuery>,
) -> Result<Json<AdminEnvelope<ListData<SocialCommentAdminItem>>>, AdminSocialError> {
    authorize(&state, &headers)?;
    let (limit, offset) = query.paging();
    let status = normalize_optional_content_status(query.status.as_deref())?;

    let total = sqlx::query_scalar::<_, i64>(
        "select count(*) from social_comments where ($1::text is null or status = $1)",
    )
    .bind(status.as_deref())
    .fetch_one(&state.db)
    .await?;

    let rows = sqlx::query_as::<_, SocialCommentAdminItem>(
        r#"
        select sc.id,
               sc.post_id,
               sc.author_user_id,
               coalesce(nullif(p.display_name, ''), 'Pengguna Spark') as author_display_name,
               sc.parent_comment_id,
               sc.body,
               sc.status,
               coalesce((
                 select jsonb_object_agg(kind, total)
                 from (
                   select sr.kind, count(*)::bigint as total
                   from social_reactions sr
                   where sr.comment_id = sc.id
                   group by sr.kind
                 ) reaction_counts
               ), '{}'::jsonb) as reactions,
               (select count(*) from social_reports r where r.target_type = 'comment' and r.target_id = sc.id) as reports_count,
               sc.created_at,
               sc.updated_at
        from social_comments sc
        left join profiles p on p.user_id = sc.author_user_id
        where ($1::text is null or sc.status = $1)
        order by sc.created_at desc, sc.id desc
        limit $2 offset $3
        "#,
    )
    .bind(status.as_deref())
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    Ok(success(ListData {
        items: rows,
        limit,
        offset,
        total,
        data_source: "database",
    }))
}

#[derive(Debug, Deserialize)]
struct ModerationActionRequest {
    target_type: String,
    target_id: Uuid,
    action: String,
    reason: Option<String>,
    report_id: Option<Uuid>,
    payload: Option<Value>,
}

#[derive(Serialize, FromRow)]
struct ModerationActionResponse {
    id: Uuid,
    moderator_user_id: Option<Uuid>,
    target_type: String,
    target_id: Uuid,
    action: String,
    reason: String,
    payload: Value,
    created_at: DateTime<Utc>,
}

async fn create_moderation_action(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ModerationActionRequest>,
) -> Result<(StatusCode, Json<AdminEnvelope<ModerationActionResponse>>), AdminSocialError> {
    authorize(&state, &headers)?;
    let target_type = normalize_moderation_target_type(&payload.target_type)?;
    let action = normalize_moderation_action(&payload.action)?;
    let reason = clean_reason(payload.reason.as_deref())?;
    let action_payload = payload.payload.unwrap_or_else(|| json!({}));
    validate_action_target_pair(&target_type, &action)?;

    let action_row = sqlx::query_as::<_, ModerationActionResponse>(
        r#"
        insert into social_moderation_actions (
          id, moderator_user_id, target_type, target_id, action, reason, payload
        )
        values ($1, null, $2, $3, $4, $5, $6)
        returning id, moderator_user_id, target_type, target_id, action, reason, payload, created_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&target_type)
    .bind(payload.target_id)
    .bind(&action)
    .bind(&reason)
    .bind(action_payload)
    .fetch_one(&state.db)
    .await?;

    apply_moderation_action(&state, &action_row, payload.report_id).await?;

    Ok((StatusCode::CREATED, success(action_row)))
}

async fn apply_moderation_action(
    state: &AppState,
    action_row: &ModerationActionResponse,
    report_id: Option<Uuid>,
) -> Result<(), AdminSocialError> {
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
            update_report_status(state, action_row.target_id, "dismissed", action_row.id).await?
        }
        ("report", "mark_reviewed") => {
            update_report_status(state, action_row.target_id, "reviewed", action_row.id).await?
        }
        _ => {
            return Err(AdminSocialError::BadRequest(
                "unsupported moderation action".to_string(),
            ))
        }
    }

    if let Some(report_id) = report_id {
        update_report_status(state, report_id, "actioned", action_row.id).await?;
    }

    Ok(())
}

async fn update_post_status(
    state: &AppState,
    post_id: Uuid,
    status: &str,
    action_id: Uuid,
) -> Result<(), AdminSocialError> {
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
        Err(AdminSocialError::NotFound("Social post was not found."))
    } else {
        Ok(())
    }
}

async fn update_comment_status(
    state: &AppState,
    comment_id: Uuid,
    status: &str,
    action_id: Uuid,
) -> Result<(), AdminSocialError> {
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
        Err(AdminSocialError::NotFound("Social comment was not found."))
    } else {
        Ok(())
    }
}

async fn update_report_status(
    state: &AppState,
    report_id: Uuid,
    status: &str,
    action_id: Uuid,
) -> Result<(), AdminSocialError> {
    let result = sqlx::query(
        r#"
        update social_reports
        set status = $2,
            action_id = $3,
            reviewed_at = coalesce(reviewed_at, now()),
            updated_at = now()
        where id = $1
        "#,
    )
    .bind(report_id)
    .bind(status)
    .bind(action_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        Err(AdminSocialError::NotFound("Social report was not found."))
    } else {
        Ok(())
    }
}

fn normalize_optional_status(input: Option<&str>) -> Result<Option<String>, AdminSocialError> {
    input
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            match value.as_str() {
                "pending" | "reviewed" | "actioned" | "dismissed" => Ok(value),
                _ => Err(AdminSocialError::BadRequest(
                    "status must be pending, reviewed, actioned, or dismissed".to_string(),
                )),
            }
        })
        .transpose()
}

fn normalize_optional_content_status(
    input: Option<&str>,
) -> Result<Option<String>, AdminSocialError> {
    input
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            match value.as_str() {
                "published" | "hidden" | "removed" | "deleted" => Ok(value),
                _ => Err(AdminSocialError::BadRequest(
                    "status must be published, hidden, removed, or deleted".to_string(),
                )),
            }
        })
        .transpose()
}

fn normalize_optional_target_type(input: Option<&str>) -> Result<Option<String>, AdminSocialError> {
    input
        .map(|value| normalize_report_target_type(value))
        .transpose()
}

fn normalize_report_target_type(input: &str) -> Result<String, AdminSocialError> {
    let value = input.trim().to_ascii_lowercase();
    match value.as_str() {
        "post" | "comment" | "profile" | "media" => Ok(value),
        _ => Err(AdminSocialError::BadRequest(
            "target_type must be post, comment, profile, or media".to_string(),
        )),
    }
}

fn normalize_moderation_target_type(input: &str) -> Result<String, AdminSocialError> {
    let value = input.trim().to_ascii_lowercase();
    match value.as_str() {
        "post" | "comment" | "report" => Ok(value),
        _ => Err(AdminSocialError::BadRequest(
            "target_type must be post, comment, or report".to_string(),
        )),
    }
}

fn normalize_moderation_action(input: &str) -> Result<String, AdminSocialError> {
    let value = input.trim().to_ascii_lowercase();
    match value.as_str() {
        "hide" | "remove" | "restore" | "dismiss_report" | "mark_reviewed" => Ok(value),
        _ => Err(AdminSocialError::BadRequest(
            "action must be hide, remove, restore, dismiss_report, or mark_reviewed".to_string(),
        )),
    }
}

fn validate_action_target_pair(target_type: &str, action: &str) -> Result<(), AdminSocialError> {
    match (target_type, action) {
        ("post" | "comment", "hide" | "remove" | "restore") => Ok(()),
        ("report", "dismiss_report" | "mark_reviewed") => Ok(()),
        _ => Err(AdminSocialError::BadRequest(
            "action is not valid for the given target_type".to_string(),
        )),
    }
}

fn clean_reason(input: Option<&str>) -> Result<String, AdminSocialError> {
    let value = input.unwrap_or("").trim();
    if value.chars().count() > 1000 {
        return Err(AdminSocialError::BadRequest(
            "reason is too long".to_string(),
        ));
    }
    if value.chars().any(char::is_control) {
        return Err(AdminSocialError::BadRequest(
            "reason cannot contain control characters".to_string(),
        ));
    }
    Ok(value.to_string())
}
