use axum::{routing::{get, post}, Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

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
        .route("/inspect", post(inspect_recovery_artifact))
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
        phase: "admin-recovery-artifact-intake-shell",
        routes: vec!["POST /api/admin/recovery/inspect"],
        policy: vec![
            "recovery artifact intake requires raw artifact token plus matching email",
            "artifact token is checked by hash only",
            "expired, used, revoked, or mismatched artifacts are rejected",
            "inspection does not mutate password, email, 2FA, or artifact status",
            "credential recovery execution remains a later separate flow",
        ],
    })
}

#[derive(Debug, Deserialize)]
struct InspectRecoveryRequest {
    token: String,
    email: String,
}

#[derive(Debug, Serialize, FromRow)]
struct RecoveryArtifactInspectRow {
    artifact_id: Uuid,
    reset_request_id: Uuid,
    email: String,
    request_type: String,
    target_role: Option<String>,
    status: String,
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct RecoveryArtifactInspectData {
    artifact_id: Uuid,
    reset_request_id: Uuid,
    email: String,
    request_type: String,
    target_role: Option<String>,
    status: String,
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    credential_mutation: bool,
}

async fn inspect_recovery_artifact(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(payload): Json<InspectRecoveryRequest>,
) -> Result<Json<AdminEnvelope<RecoveryArtifactInspectData>>, ApiError> {
    let token_hash = hash_token(normalize_token(&payload.token)?.as_str());
    let email = normalize_email(&payload.email)?;

    let artifact = sqlx::query_as::<_, RecoveryArtifactInspectRow>(
        r#"
        select id as artifact_id,
               reset_request_id,
               email,
               request_type,
               target_role,
               status,
               issued_at,
               expires_at
        from admin_recovery_artifacts
        where token_hash = $1
          and lower(email) = lower($2)
          and status = 'pending'
          and expires_at > now()
          and used_at is null
          and revoked_at is null
        "#,
    )
    .bind(token_hash)
    .bind(&email)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("recovery artifact is invalid or expired".to_string()))?;

    Ok(success(RecoveryArtifactInspectData {
        artifact_id: artifact.artifact_id,
        reset_request_id: artifact.reset_request_id,
        email: artifact.email,
        request_type: artifact.request_type,
        target_role: artifact.target_role,
        status: artifact.status,
        issued_at: artifact.issued_at,
        expires_at: artifact.expires_at,
        credential_mutation: false,
    }))
}

fn normalize_token(input: &str) -> Result<String, ApiError> {
    let token = input.trim();
    if !token.starts_with("adm_rec_") || token.len() < 72 || token.len() > 96 || token.chars().any(char::is_whitespace) {
        return Err(ApiError::BadRequest("recovery artifact is invalid or expired".to_string()));
    }
    Ok(token.to_string())
}

fn normalize_email(input: &str) -> Result<String, ApiError> {
    let value = input.trim().to_ascii_lowercase();
    if value.len() < 3 || value.len() > 254 || !value.contains('@') || value.contains(' ') {
        return Err(ApiError::BadRequest("recovery artifact is invalid or expired".to_string()));
    }
    Ok(value)
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}
