use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{admin_auth, error::ApiError, state::AppState};

const RESET_REQUEST_DAYS: i64 = 7;

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

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/scope", get(scope))
        .route("/request", post(create_reset_request))
        .route("/requests", get(reset_requests))
        .route("/requests/:request_id/review", post(review_reset_request))
}

#[derive(Serialize)]
struct ScopeData {
    module: &'static str,
    phase: &'static str,
    routes: Vec<&'static str>,
    policy: Vec<&'static str>,
}

async fn scope() -> Json<AdminEnvelope<ScopeData>> {
    success(ScopeData {
        module: module_path!(),
        phase: "admin-reset-request-boundary",
        routes: vec![
            "POST /api/admin/reset/request",
            "GET /api/admin/reset/requests",
            "POST /api/admin/reset/requests/:request_id/review",
        ],
        policy: vec![
            "public reset request endpoint always returns neutral response",
            "password/email/totp reset requests do not confirm account existence",
            "review queue requires admin_manage capability",
            "this pass records approval/rejection only; actual credential reset remains a separate controlled action",
        ],
    })
}

#[derive(Debug, Deserialize)]
struct ResetRequestPayload {
    email: String,
    request_type: String,
    note: Option<String>,
}

#[derive(Serialize)]
struct ResetRequestReceipt {
    status: &'static str,
    message: &'static str,
}

#[derive(Debug, Deserialize)]
struct ResetRequestsQuery {
    status: Option<String>,
    request_type: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Serialize, FromRow)]
struct ResetRequestRow {
    id: Uuid,
    email: String,
    request_type: String,
    status: String,
    requested_at: DateTime<Utc>,
    reviewed_by_actor_kind: Option<String>,
    reviewed_by_user_id: Option<Uuid>,
    reviewed_at: Option<DateTime<Utc>>,
    expires_at: DateTime<Utc>,
    metadata: Value,
}

#[derive(Serialize)]
struct ResetRequestsData {
    items: Vec<ResetRequestRow>,
    total: i64,
    limit: i64,
    offset: i64,
}

#[derive(Debug, Deserialize)]
struct ReviewResetRequestPayload {
    decision: String,
    reason: Option<String>,
}

#[derive(Serialize)]
struct ReviewResetRequestData {
    request: ResetRequestRow,
}

async fn create_reset_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ResetRequestPayload>,
) -> Result<Json<AdminEnvelope<ResetRequestReceipt>>, ApiError> {
    let email = normalize_email(&payload.email).ok();
    let request_type = normalize_request_type(&payload.request_type).ok();

    if let (Some(email), Some(request_type)) = (email, request_type) {
        let note = payload
            .note
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.chars().take(500).collect::<String>());
        let expires_at = Utc::now() + Duration::days(RESET_REQUEST_DAYS);
        let ip_hash = header_hash(&headers, "x-forwarded-for")
            .or_else(|| header_hash(&headers, "x-real-ip"));
        let ua_hash = header_hash(&headers, "user-agent");

        let duplicate = sqlx::query_scalar::<_, bool>(
            r#"
            select exists(
              select 1 from admin_reset_requests
              where lower(email) = lower($1)
                and request_type = $2
                and status = 'pending'
                and expires_at > now()
            )
            "#,
        )
        .bind(&email)
        .bind(&request_type)
        .fetch_one(&state.db)
        .await?;

        if !duplicate {
            sqlx::query(
                r#"
                insert into admin_reset_requests (
                  id, email, request_type, requested_by_ip_hash, user_agent_hash, expires_at, metadata
                ) values ($1, $2, $3, $4, $5, $6, $7)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(&email)
            .bind(&request_type)
            .bind(ip_hash)
            .bind(ua_hash)
            .bind(expires_at)
            .bind(json!({
                "source": "admin_reset_request_public",
                "note": note,
                "neutral_response": true
            }))
            .execute(&state.db)
            .await?;
        }
    }

    Ok(success(ResetRequestReceipt {
        status: "received",
        message: "If this email is eligible for admin recovery, the request will be reviewed through the approved internal channel.",
    }))
}

async fn reset_requests(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ResetRequestsQuery>,
) -> Result<Json<AdminEnvelope<ResetRequestsData>>, ApiError> {
    admin_auth::authorize_admin_manage(&state, &headers).await?;
    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);
    let status = query
        .status
        .as_deref()
        .map(normalize_status)
        .transpose()?
        .unwrap_or_else(|| "pending".to_string());
    let request_type = query
        .request_type
        .as_deref()
        .map(normalize_request_type)
        .transpose()?;

    let items = sqlx::query_as::<_, ResetRequestRow>(
        r#"
        select id, email, request_type, status, requested_at,
               reviewed_by_actor_kind, reviewed_by_user_id, reviewed_at,
               expires_at, metadata
        from admin_reset_requests
        where ($1::text = 'all' or status = $1)
          and ($2::text is null or request_type = $2)
        order by requested_at desc
        limit $3 offset $4
        "#,
    )
    .bind(&status)
    .bind(request_type.as_deref())
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total = sqlx::query_scalar::<_, i64>(
        r#"
        select count(*) from admin_reset_requests
        where ($1::text = 'all' or status = $1)
          and ($2::text is null or request_type = $2)
        "#,
    )
    .bind(&status)
    .bind(request_type.as_deref())
    .fetch_one(&state.db)
    .await?;

    Ok(success(ResetRequestsData {
        items,
        total,
        limit,
        offset,
    }))
}

async fn review_reset_request(
    Path(request_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ReviewResetRequestPayload>,
) -> Result<Json<AdminEnvelope<ReviewResetRequestData>>, ApiError> {
    let actor = admin_auth::authorize_admin_manage(&state, &headers).await?;
    let decision = normalize_decision(&payload.decision)?;
    let reason = payload
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("reviewed by admin reset queue");

    let request = sqlx::query_as::<_, ResetRequestRow>(
        r#"
        update admin_reset_requests
        set status = $2,
            reviewed_by_actor_kind = $3,
            reviewed_by_user_id = $4,
            reviewed_at = now(),
            metadata = metadata || $5::jsonb
        where id = $1
          and status = 'pending'
          and expires_at > now()
        returning id, email, request_type, status, requested_at,
                  reviewed_by_actor_kind, reviewed_by_user_id, reviewed_at,
                  expires_at, metadata
        "#,
    )
    .bind(request_id)
    .bind(&decision)
    .bind(&actor.actor_kind)
    .bind(actor.actor_user_id)
    .bind(json!({"reason": reason, "reviewed_via": "admin_reset_queue"}))
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("reset request is not pending or has expired".to_string()))?;

    admin_auth::audit(
        &state,
        &actor,
        "admin_reset_request_review",
        "admin_reset_request",
        request.reviewed_by_user_id,
        Some(request.id),
        &[],
        "Admin reset request was reviewed.",
        json!({"request_type": request.request_type, "status": request.status, "reason": reason}),
    )
    .await?;

    Ok(success(ReviewResetRequestData { request }))
}

fn normalize_email(input: &str) -> Result<String, ApiError> {
    let value = input.trim().to_ascii_lowercase();
    if value.len() < 3 || value.len() > 254 || !value.contains('@') {
        return Err(ApiError::BadRequest("email is invalid".to_string()));
    }
    Ok(value)
}

fn normalize_request_type(input: &str) -> Result<String, ApiError> {
    match input.trim().to_ascii_lowercase().as_str() {
        "password" | "email" | "totp" => Ok(input.trim().to_ascii_lowercase()),
        _ => Err(ApiError::BadRequest("request_type is invalid".to_string())),
    }
}

fn normalize_status(input: &str) -> Result<String, ApiError> {
    match input.trim().to_ascii_lowercase().as_str() {
        "pending" | "approved" | "rejected" | "completed" | "expired" | "all" => Ok(input.trim().to_ascii_lowercase()),
        _ => Err(ApiError::BadRequest("status is invalid".to_string())),
    }
}

fn normalize_decision(input: &str) -> Result<String, ApiError> {
    match input.trim().to_ascii_lowercase().as_str() {
        "approved" | "rejected" => Ok(input.trim().to_ascii_lowercase()),
        _ => Err(ApiError::BadRequest("decision must be approved or rejected".to_string())),
    }
}

fn header_hash(headers: &HeaderMap, name: &str) -> Option<String> {
    let value = headers.get(name)?.to_str().ok()?.trim();
    if value.is_empty() {
        return None;
    }
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    Some(format!("{:x}", hasher.finalize()))
}
