use std::convert::TryInto;

use aes_gcm::{
    aead::{rand_core::RngCore, Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{routing::{get, post}, Json, Router};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use chrono::{DateTime, Utc};
use data_encoding::BASE32_NOPAD;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha1::Sha1;
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{admin_auth, error::ApiError, state::AppState};

type HmacSha1 = Hmac<Sha1>;

const MIN_PASSWORD_LEN: usize = 8;
const MAX_PASSWORD_LEN: usize = 128;
const TOTP_STEP_SECONDS: i64 = 30;

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
        .route("/password", post(execute_password_recovery))
        .route("/totp/setup", post(setup_totp_recovery))
        .route("/totp/confirm", post(confirm_totp_recovery))
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
        phase: "admin-2fa-recovery-rotation-flow",
        routes: vec![
            "POST /api/admin/recovery/inspect",
            "POST /api/admin/recovery/password",
            "POST /api/admin/recovery/totp/setup",
            "POST /api/admin/recovery/totp/confirm",
        ],
        policy: vec![
            "recovery artifact intake requires raw artifact token plus matching email",
            "artifact token is checked by hash only",
            "expired, used, revoked, or mismatched artifacts are rejected",
            "inspection does not mutate password, email, 2FA, or artifact status",
            "password recovery requires an approved password recovery artifact plus current TOTP code",
            "password recovery consumes the artifact exactly once",
            "2FA recovery requires approved 2FA artifact plus account password",
            "2FA recovery creates a fresh pending TOTP factor before revoking the old factor",
            "2FA recovery only revokes old factors after the new TOTP code is confirmed",
            "2FA recovery consumes the artifact and completes the reset request exactly once",
            "email recovery execution remains a separate future flow",
        ],
    })
}

#[derive(Debug, Deserialize)]
struct InspectRecoveryRequest {
    token: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct PasswordRecoveryRequest {
    token: String,
    email: String,
    new_password: String,
    totp_code: String,
}

#[derive(Debug, Deserialize)]
struct TotpRecoverySetupRequest {
    token: String,
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct TotpRecoveryConfirmRequest {
    token: String,
    email: String,
    password: String,
    factor_id: Uuid,
    code: String,
}

#[derive(Debug, Clone, FromRow)]
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

#[derive(Debug, FromRow)]
struct RecoveryPasswordTargetRow {
    user_id: Uuid,
    email: String,
    role: String,
    capabilities: Vec<String>,
    factor_id: Uuid,
    secret_ciphertext: Vec<u8>,
    secret_nonce: Vec<u8>,
    last_used_step: Option<i64>,
}

#[derive(Debug, FromRow)]
struct RecoveryCredentialTargetRow {
    user_id: Uuid,
    email: String,
    password_hash: Option<String>,
    role: String,
    capabilities: Vec<String>,
}

#[derive(Debug, FromRow)]
struct RecoveryPendingTotpFactorRow {
    id: Uuid,
    secret_ciphertext: Vec<u8>,
    secret_nonce: Vec<u8>,
    last_used_step: Option<i64>,
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

#[derive(Serialize)]
struct PasswordRecoveryData {
    artifact_id: Uuid,
    reset_request_id: Uuid,
    email: String,
    target_role: Option<String>,
    password_changed_at: DateTime<Utc>,
    reset_request_completed: bool,
    sessions_revoked: bool,
}

#[derive(Serialize)]
struct TotpRecoverySetupData {
    artifact_id: Uuid,
    reset_request_id: Uuid,
    factor_id: Uuid,
    issuer: &'static str,
    account_name: String,
    otpauth_uri: String,
    manual_secret: String,
    old_factor_revoked: bool,
}

#[derive(Serialize)]
struct TotpRecoveryConfirmData {
    artifact_id: Uuid,
    reset_request_id: Uuid,
    factor_id: Uuid,
    email: String,
    target_role: Option<String>,
    enabled_at: DateTime<Utc>,
    old_factors_revoked: bool,
    reset_request_completed: bool,
    sessions_revoked: bool,
}

async fn inspect_recovery_artifact(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(payload): Json<InspectRecoveryRequest>,
) -> Result<Json<AdminEnvelope<RecoveryArtifactInspectData>>, ApiError> {
    let artifact = load_pending_artifact(&state, &payload.token, &payload.email).await?;

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

async fn execute_password_recovery(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(payload): Json<PasswordRecoveryRequest>,
) -> Result<Json<AdminEnvelope<PasswordRecoveryData>>, ApiError> {
    let artifact = load_pending_artifact(&state, &payload.token, &payload.email).await?;
    if artifact.request_type != "password" {
        return Err(ApiError::BadRequest("recovery artifact is not valid for password recovery".to_string()));
    }
    validate_password(&payload.new_password)?;
    let password_hash = hash_password(&payload.new_password)?;

    let target = load_password_recovery_target(&state, &artifact.email, artifact.target_role.as_deref()).await?;
    let secret = decrypt_totp_secret(&target.secret_ciphertext, &target.secret_nonce)?;
    let used_step = verify_totp_code(&secret, &payload.totp_code, target.last_used_step)?;
    let changed_at = Utc::now();

    let mut tx = state.db.begin().await?;

    sqlx::query(
        r#"
        update admin_totp_factors
        set last_used_step = $2, updated_at = now()
        where id = $1
          and enabled_at is not null
          and revoked_at is null
        "#,
    )
    .bind(target.factor_id)
    .bind(used_step)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        update users
        set password_hash = $2
        where id = $1
          and status = 'active'
        "#,
    )
    .bind(target.user_id)
    .bind(&password_hash)
    .execute(&mut *tx)
    .await?;

    consume_artifact_and_complete_request_tx(
        &mut tx,
        &artifact,
        changed_at,
        json!({
            "credential_mutation": true,
            "mutation_type": "password",
            "target_user_id": target.user_id,
            "target_role": target.role
        }),
        json!({
            "completed_at": changed_at,
            "completed_via": "admin_password_recovery",
            "artifact_id": artifact.artifact_id,
            "credential_mutation": true,
            "mutation_type": "password"
        }),
    )
    .await?;

    revoke_admin_sessions_tx(&mut tx, target.user_id, changed_at).await?;

    tx.commit().await?;

    let actor = admin_auth::AdminContext {
        actor_kind: format!("{}_password_recovery", target.role),
        actor_user_id: Some(target.user_id),
        role: target.role.clone(),
        capabilities: admin_auth::sanitize_capabilities_for_role(&target.role, &target.capabilities),
    };

    admin_auth::audit(
        &state,
        &actor,
        "admin_recovery_password_completed",
        "user",
        Some(target.user_id),
        Some(artifact.artifact_id),
        &actor.capabilities,
        "Admin password was recovered through approved recovery artifact flow.",
        json!({
            "reset_request_id": artifact.reset_request_id,
            "artifact_id": artifact.artifact_id,
            "target_role": artifact.target_role,
            "reset_request_completed": true,
            "sessions_revoked": true,
            "credential_mutation": true
        }),
    )
    .await?;

    Ok(success(PasswordRecoveryData {
        artifact_id: artifact.artifact_id,
        reset_request_id: artifact.reset_request_id,
        email: target.email,
        target_role: artifact.target_role,
        password_changed_at: changed_at,
        reset_request_completed: true,
        sessions_revoked: true,
    }))
}

async fn setup_totp_recovery(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(payload): Json<TotpRecoverySetupRequest>,
) -> Result<Json<AdminEnvelope<TotpRecoverySetupData>>, ApiError> {
    let artifact = load_pending_artifact(&state, &payload.token, &payload.email).await?;
    if artifact.request_type != "totp" {
        return Err(ApiError::BadRequest("recovery artifact is not valid for 2FA recovery".to_string()));
    }

    let target = load_credential_recovery_target(&state, &artifact.email, artifact.target_role.as_deref()).await?;
    let password_hash = target.password_hash.as_deref().ok_or(ApiError::Unauthorized)?;
    if !verify_password(password_hash, &payload.password) {
        return Err(ApiError::Unauthorized);
    }

    let mut secret = [0_u8; 20];
    OsRng.fill_bytes(&mut secret);
    let manual_secret = BASE32_NOPAD.encode(&secret);
    let (ciphertext, nonce) = encrypt_totp_secret(&secret)?;
    let factor_id = Uuid::new_v4();

    let mut tx = state.db.begin().await?;

    sqlx::query(
        r#"
        update admin_totp_factors
        set revoked_at = coalesce(revoked_at, now()),
            metadata = metadata || $3::jsonb,
            updated_at = now()
        where user_id = $1
          and enabled_at is null
          and revoked_at is null
          and metadata->>'recovery_artifact_id' = $2
        "#,
    )
    .bind(target.user_id)
    .bind(artifact.artifact_id.to_string())
    .bind(json!({"status": "superseded_by_new_2fa_recovery_setup"}))
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        insert into admin_totp_factors (
          id, user_id, label, secret_ciphertext, secret_nonce, metadata
        ) values ($1, $2, 'Karyra Spark Admin Recovery', $3, $4, $5)
        "#,
    )
    .bind(factor_id)
    .bind(target.user_id)
    .bind(ciphertext)
    .bind(nonce)
    .bind(json!({
        "source": "admin_totp_recovery",
        "status": "pending_confirmation",
        "recovery_artifact_id": artifact.artifact_id,
        "reset_request_id": artifact.reset_request_id,
        "target_role": target.role
    }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let actor = recovery_actor(target.user_id, &target.role, &target.capabilities, "totp_recovery_setup");

    admin_auth::audit(
        &state,
        &actor,
        "admin_recovery_totp_setup_started",
        "admin_totp_factor",
        Some(target.user_id),
        Some(factor_id),
        &actor.capabilities,
        "Admin 2FA recovery setup was started through approved recovery artifact flow.",
        json!({
            "reset_request_id": artifact.reset_request_id,
            "artifact_id": artifact.artifact_id,
            "target_role": artifact.target_role,
            "old_factor_revoked": false
        }),
    )
    .await?;

    let account_name = target.email;
    let otpauth_uri = format!(
        "otpauth://totp/Karyra%20Spark:{}?secret={}&issuer=Karyra%20Spark&algorithm=SHA1&digits=6&period=30",
        url_component(&account_name),
        manual_secret
    );

    Ok(success(TotpRecoverySetupData {
        artifact_id: artifact.artifact_id,
        reset_request_id: artifact.reset_request_id,
        factor_id,
        issuer: "Karyra Spark",
        account_name,
        otpauth_uri,
        manual_secret,
        old_factor_revoked: false,
    }))
}

async fn confirm_totp_recovery(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(payload): Json<TotpRecoveryConfirmRequest>,
) -> Result<Json<AdminEnvelope<TotpRecoveryConfirmData>>, ApiError> {
    let artifact = load_pending_artifact(&state, &payload.token, &payload.email).await?;
    if artifact.request_type != "totp" {
        return Err(ApiError::BadRequest("recovery artifact is not valid for 2FA recovery".to_string()));
    }

    let target = load_credential_recovery_target(&state, &artifact.email, artifact.target_role.as_deref()).await?;
    let password_hash = target.password_hash.as_deref().ok_or(ApiError::Unauthorized)?;
    if !verify_password(password_hash, &payload.password) {
        return Err(ApiError::Unauthorized);
    }

    let pending = load_pending_recovery_totp_factor(&state, target.user_id, payload.factor_id, artifact.artifact_id).await?;
    let secret = decrypt_totp_secret(&pending.secret_ciphertext, &pending.secret_nonce)?;
    let used_step = verify_totp_code(&secret, &payload.code, pending.last_used_step)?;
    let enabled_at = Utc::now();

    let mut tx = state.db.begin().await?;

    sqlx::query(
        r#"
        update admin_totp_factors
        set revoked_at = coalesce(revoked_at, $2),
            metadata = metadata || $3::jsonb,
            updated_at = now()
        where user_id = $1
          and id <> $4
          and enabled_at is not null
          and revoked_at is null
        "#,
    )
    .bind(target.user_id)
    .bind(enabled_at)
    .bind(json!({
        "revoked_via": "admin_totp_recovery_rotation",
        "recovery_artifact_id": artifact.artifact_id,
        "replacement_factor_id": pending.id
    }))
    .bind(pending.id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        update admin_totp_factors
        set enabled_at = $2,
            verified_at = $2,
            last_used_step = $3,
            updated_at = now(),
            metadata = metadata || $4::jsonb
        where id = $1
          and user_id = $5
          and enabled_at is null
          and revoked_at is null
        "#,
    )
    .bind(pending.id)
    .bind(enabled_at)
    .bind(used_step)
    .bind(json!({
        "status": "enabled",
        "enabled_via": "admin_totp_recovery_rotation",
        "recovery_artifact_id": artifact.artifact_id
    }))
    .bind(target.user_id)
    .execute(&mut *tx)
    .await?;

    consume_artifact_and_complete_request_tx(
        &mut tx,
        &artifact,
        enabled_at,
        json!({
            "credential_mutation": true,
            "mutation_type": "totp_rotation",
            "target_user_id": target.user_id,
            "target_role": target.role,
            "new_factor_id": pending.id
        }),
        json!({
            "completed_at": enabled_at,
            "completed_via": "admin_totp_recovery_rotation",
            "artifact_id": artifact.artifact_id,
            "credential_mutation": true,
            "mutation_type": "totp_rotation",
            "new_factor_id": pending.id
        }),
    )
    .await?;

    revoke_admin_sessions_tx(&mut tx, target.user_id, enabled_at).await?;

    tx.commit().await?;

    let actor = recovery_actor(target.user_id, &target.role, &target.capabilities, "totp_recovery_confirm");

    admin_auth::audit(
        &state,
        &actor,
        "admin_recovery_totp_completed",
        "admin_totp_factor",
        Some(target.user_id),
        Some(pending.id),
        &actor.capabilities,
        "Admin 2FA was rotated through approved recovery artifact flow.",
        json!({
            "reset_request_id": artifact.reset_request_id,
            "artifact_id": artifact.artifact_id,
            "target_role": artifact.target_role,
            "old_factors_revoked": true,
            "reset_request_completed": true,
            "sessions_revoked": true,
            "credential_mutation": true
        }),
    )
    .await?;

    Ok(success(TotpRecoveryConfirmData {
        artifact_id: artifact.artifact_id,
        reset_request_id: artifact.reset_request_id,
        factor_id: pending.id,
        email: target.email,
        target_role: artifact.target_role,
        enabled_at,
        old_factors_revoked: true,
        reset_request_completed: true,
        sessions_revoked: true,
    }))
}

async fn consume_artifact_and_complete_request_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    artifact: &RecoveryArtifactInspectRow,
    completed_at: DateTime<Utc>,
    artifact_metadata: serde_json::Value,
    reset_metadata: serde_json::Value,
) -> Result<(), ApiError> {
    let consumed = sqlx::query_scalar::<_, Option<Uuid>>(
        r#"
        update admin_recovery_artifacts
        set status = 'used',
            used_at = $2,
            metadata = metadata || $3::jsonb
        where id = $1
          and status = 'pending'
          and expires_at > now()
          and used_at is null
          and revoked_at is null
        returning id
        "#,
    )
    .bind(artifact.artifact_id)
    .bind(completed_at)
    .bind(artifact_metadata)
    .fetch_optional(&mut **tx)
    .await?
    .flatten();

    if consumed.is_none() {
        return Err(ApiError::Conflict("recovery artifact is no longer pending".to_string()));
    }

    let completed = sqlx::query_scalar::<_, Option<Uuid>>(
        r#"
        update admin_reset_requests
        set status = 'completed',
            metadata = metadata || $3::jsonb
        where id = $1
          and status = 'approved'
          and expires_at > now()
        returning id
        "#,
    )
    .bind(artifact.reset_request_id)
    .bind(completed_at)
    .bind(reset_metadata)
    .fetch_optional(&mut **tx)
    .await?
    .flatten();

    if completed.is_none() {
        return Err(ApiError::Conflict("reset request is no longer approved".to_string()));
    }

    Ok(())
}

async fn revoke_admin_sessions_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    revoked_at: DateTime<Utc>,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        update admin_sessions
        set revoked_at = coalesce(revoked_at, $2),
            last_seen_at = now()
        where user_id = $1
          and revoked_at is null
        "#,
    )
    .bind(user_id)
    .bind(revoked_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn load_pending_artifact(
    state: &AppState,
    token: &str,
    email: &str,
) -> Result<RecoveryArtifactInspectRow, ApiError> {
    let token_hash = hash_token(normalize_token(token)?.as_str());
    let email = normalize_email(email)?;

    sqlx::query_as::<_, RecoveryArtifactInspectRow>(
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
    .ok_or_else(|| ApiError::BadRequest("recovery artifact is invalid or expired".to_string()))
}

async fn load_password_recovery_target(
    state: &AppState,
    email: &str,
    target_role: Option<&str>,
) -> Result<RecoveryPasswordTargetRow, ApiError> {
    sqlx::query_as::<_, RecoveryPasswordTargetRow>(
        r#"
        select users.id as user_id,
               users.email,
               case when ara.role = 'sub_admin' then 'admin' else ara.role end as role,
               ara.capabilities,
               factor.id as factor_id,
               factor.secret_ciphertext,
               factor.secret_nonce,
               factor.last_used_step
        from users
        join admin_role_assignments ara on ara.user_id = users.id
        join lateral (
          select id, secret_ciphertext, secret_nonce, last_used_step
          from admin_totp_factors
          where user_id = users.id
            and enabled_at is not null
            and revoked_at is null
          order by enabled_at desc
          limit 1
        ) factor on true
        where lower(users.email) = lower($1)
          and users.status = 'active'
          and ($2::text is null or (case when ara.role = 'sub_admin' then 'admin' else ara.role end) = $2)
          and ara.role in ('admin', 'sub_admin', 'moderator')
          and ara.status = 'active'
          and ara.revoked_at is null
          and ara.starts_at <= now()
          and (ara.expires_at is null or ara.expires_at > now())
        order by case ara.role when 'admin' then 2 when 'sub_admin' then 2 when 'moderator' then 1 else 0 end desc,
                 ara.updated_at desc
        limit 1
        "#,
    )
    .bind(email)
    .bind(target_role)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("recovery artifact is invalid or expired".to_string()))
}

async fn load_credential_recovery_target(
    state: &AppState,
    email: &str,
    target_role: Option<&str>,
) -> Result<RecoveryCredentialTargetRow, ApiError> {
    sqlx::query_as::<_, RecoveryCredentialTargetRow>(
        r#"
        select users.id as user_id,
               users.email,
               users.password_hash,
               case when ara.role = 'sub_admin' then 'admin' else ara.role end as role,
               ara.capabilities
        from users
        join admin_role_assignments ara on ara.user_id = users.id
        where lower(users.email) = lower($1)
          and users.status = 'active'
          and ($2::text is null or (case when ara.role = 'sub_admin' then 'admin' else ara.role end) = $2)
          and ara.role in ('admin', 'sub_admin', 'moderator')
          and ara.status = 'active'
          and ara.revoked_at is null
          and ara.starts_at <= now()
          and (ara.expires_at is null or ara.expires_at > now())
        order by case ara.role when 'admin' then 2 when 'sub_admin' then 2 when 'moderator' then 1 else 0 end desc,
                 ara.updated_at desc
        limit 1
        "#,
    )
    .bind(email)
    .bind(target_role)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("recovery artifact is invalid or expired".to_string()))
}

async fn load_pending_recovery_totp_factor(
    state: &AppState,
    user_id: Uuid,
    factor_id: Uuid,
    artifact_id: Uuid,
) -> Result<RecoveryPendingTotpFactorRow, ApiError> {
    sqlx::query_as::<_, RecoveryPendingTotpFactorRow>(
        r#"
        select id, secret_ciphertext, secret_nonce, last_used_step
        from admin_totp_factors
        where id = $1
          and user_id = $2
          and enabled_at is null
          and revoked_at is null
          and metadata->>'recovery_artifact_id' = $3
        "#,
    )
    .bind(factor_id)
    .bind(user_id)
    .bind(artifact_id.to_string())
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::Unauthorized)
}

fn recovery_actor(user_id: Uuid, role: &str, capabilities: &[String], suffix: &str) -> admin_auth::AdminContext {
    let role = admin_auth::canonical_role(role);
    admin_auth::AdminContext {
        actor_kind: format!("{role}_{suffix}"),
        actor_user_id: Some(user_id),
        role: role.clone(),
        capabilities: admin_auth::sanitize_capabilities_for_role(&role, capabilities),
    }
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

fn validate_password(password: &str) -> Result<(), ApiError> {
    let len = password.chars().count();
    if !(MIN_PASSWORD_LEN..=MAX_PASSWORD_LEN).contains(&len) {
        return Err(ApiError::BadRequest(format!(
            "password must be between {MIN_PASSWORD_LEN} and {MAX_PASSWORD_LEN} characters"
        )));
    }
    Ok(())
}

fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::encode_b64(Uuid::new_v4().as_bytes()).map_err(|error| {
        tracing::error!(?error, "failed to create admin recovery password salt");
        ApiError::Internal
    })?;

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| {
            tracing::error!(?error, "failed to hash recovered admin password");
            ApiError::Internal
        })
}

fn verify_password(password_hash: &str, password: &str) -> bool {
    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(hash) => hash,
        Err(error) => {
            tracing::warn!(?error, "stored recovery target password hash is invalid");
            return false;
        }
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

fn clean_totp_code(input: &str) -> Result<String, ApiError> {
    let code: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        return Err(ApiError::BadRequest("TOTP code must be 6 digits".to_string()));
    }
    Ok(code)
}

fn admin_mfa_key() -> Result<[u8; 32], ApiError> {
    let raw = std::env::var("SPARK_ADMIN_MFA_KEY")
        .map_err(|_| ApiError::ServiceUnavailable("SPARK_ADMIN_MFA_KEY is required for admin 2FA".to_string()))?;
    let trimmed = raw.trim();
    let decoded = if trimmed.len() == 64 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        decode_hex(trimmed)?
    } else {
        BASE64_STANDARD
            .decode(trimmed)
            .map_err(|_| ApiError::BadRequest("SPARK_ADMIN_MFA_KEY must be 32 bytes encoded as hex or base64".to_string()))?
    };
    decoded
        .try_into()
        .map_err(|_| ApiError::BadRequest("SPARK_ADMIN_MFA_KEY must decode to exactly 32 bytes".to_string()))
}

fn decode_hex(input: &str) -> Result<Vec<u8>, ApiError> {
    let mut bytes = Vec::with_capacity(input.len() / 2);
    let chars = input.as_bytes().chunks_exact(2);
    for chunk in chars {
        let text = std::str::from_utf8(chunk).map_err(|_| ApiError::BadRequest("invalid hex key".to_string()))?;
        let byte = u8::from_str_radix(text, 16).map_err(|_| ApiError::BadRequest("invalid hex key".to_string()))?;
        bytes.push(byte);
    }
    Ok(bytes)
}

fn encrypt_totp_secret(secret: &[u8]) -> Result<(Vec<u8>, Vec<u8>), ApiError> {
    let key = admin_mfa_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| ApiError::Internal)?;
    let mut nonce_bytes = [0_u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), secret)
        .map_err(|error| {
            tracing::error!(?error, "failed to encrypt admin recovery TOTP secret");
            ApiError::Internal
        })?;
    Ok((ciphertext, nonce_bytes.to_vec()))
}

fn decrypt_totp_secret(ciphertext: &[u8], nonce: &[u8]) -> Result<Vec<u8>, ApiError> {
    let key = admin_mfa_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| ApiError::Internal)?;
    let secret = cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|error| {
            tracing::warn!(?error, "failed to decrypt admin recovery TOTP secret");
            ApiError::Unauthorized
        })?;
    Ok(secret)
}

fn verify_totp_code(secret: &[u8], input: &str, last_used_step: Option<i64>) -> Result<i64, ApiError> {
    let code = clean_totp_code(input)?;
    let now_step = Utc::now().timestamp() / TOTP_STEP_SECONDS;
    for step in (now_step - 1)..=(now_step + 1) {
        if last_used_step.is_some_and(|last| step <= last) {
            continue;
        }
        if hotp(secret, step as u64)? == code {
            return Ok(step);
        }
    }
    Err(ApiError::Unauthorized)
}

fn hotp(secret: &[u8], counter: u64) -> Result<String, ApiError> {
    let mut mac = <HmacSha1 as Mac>::new_from_slice(secret).map_err(|_| ApiError::Internal)?;
    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let offset = (digest[19] & 0x0f) as usize;
    let binary = ((digest[offset] as u32 & 0x7f) << 24)
        | ((digest[offset + 1] as u32) << 16)
        | ((digest[offset + 2] as u32) << 8)
        | (digest[offset + 3] as u32);
    Ok(format!("{:06}", binary % 1_000_000))
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn url_component(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => vec![byte as char],
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}
