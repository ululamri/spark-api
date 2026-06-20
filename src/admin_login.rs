use std::convert::TryInto;

use aes_gcm::{
    aead::{rand_core::RngCore, Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::{
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2,
};
use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use data_encoding::BASE32_NOPAD;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha1::Sha1;
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{admin_auth, auth::session::read_session_cookie, error::ApiError, state::AppState};

type HmacSha1 = Hmac<Sha1>;

const ADMIN_SESSION_HOURS: i64 = 8;
const EMAIL_TOKEN_MINUTES: i64 = 20;
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
        .route("/login", post(login))
        .route("/email/request", post(request_email_verification))
        .route("/email/confirm", post(confirm_email_verification))
        .route("/totp/setup", post(setup_totp))
        .route("/totp/confirm", post(confirm_totp))
        .route("/me", get(me))
        .route("/logout", post(logout))
}

#[derive(Serialize)]
struct ScopeData {
    module: &'static str,
    phase: &'static str,
    routes: Vec<&'static str>,
    cookie_name: &'static str,
    auth_model: &'static str,
    security_gates: Vec<&'static str>,
}

async fn scope() -> Json<AdminEnvelope<ScopeData>> {
    success(ScopeData {
        module: module_path!(),
        phase: "delegated-admin-email-mfa-flow",
        routes: vec![
            "POST /api/admin/auth/login",
            "POST /api/admin/auth/email/request",
            "POST /api/admin/auth/email/confirm",
            "POST /api/admin/auth/totp/setup",
            "POST /api/admin/auth/totp/confirm",
            "GET /api/admin/auth/me",
            "POST /api/admin/auth/logout",
        ],
        cookie_name: admin_auth::ADMIN_SESSION_COOKIE_NAME,
        auth_model: "superadmin remains root-token/session based; admin/moderator use dedicated admin session cookie",
        security_gates: vec![
            "delegated admin email must be verified",
            "delegated admin TOTP factor must be enabled",
            "login requires password plus TOTP code after setup",
            "no delegated admin session is created before all gates pass",
        ],
    })
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
    totp_code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EmailRequest {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct EmailConfirmRequest {
    email: String,
    password: String,
    token: String,
}

#[derive(Debug, Deserialize)]
struct TotpSetupRequest {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct TotpConfirmRequest {
    email: String,
    password: String,
    factor_id: Uuid,
    code: String,
}

#[derive(Debug, FromRow)]
struct AdminPasswordRow {
    user_id: Uuid,
    password_hash: Option<String>,
    email: String,
    email_verified_at: Option<DateTime<Utc>>,
    role: String,
    capabilities: Vec<String>,
}

#[derive(Debug, FromRow)]
struct LoginAdminRow {
    user_id: Uuid,
    password_hash: Option<String>,
    email: String,
    email_verified_at: Option<DateTime<Utc>>,
    role: String,
    capabilities: Vec<String>,
    totp_factor_id: Option<Uuid>,
    secret_ciphertext: Option<Vec<u8>>,
    secret_nonce: Option<Vec<u8>>,
    last_used_step: Option<i64>,
}

#[derive(Debug, FromRow)]
struct TotpFactorRow {
    id: Uuid,
    secret_ciphertext: Vec<u8>,
    secret_nonce: Vec<u8>,
    last_used_step: Option<i64>,
}

#[derive(Serialize)]
struct AdminAuthData {
    actor: admin_auth::AdminContext,
    expires_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct EmailRequestData {
    email: String,
    expires_at: DateTime<Utc>,
    delivery_mode: &'static str,
    manual_token: Option<String>,
}

#[derive(Serialize)]
struct EmailConfirmData {
    email: String,
    verified_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct TotpSetupData {
    factor_id: Uuid,
    issuer: &'static str,
    account_name: String,
    otpauth_uri: String,
    manual_secret: String,
}

#[derive(Serialize)]
struct TotpConfirmData {
    factor_id: Uuid,
    enabled_at: DateTime<Utc>,
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Response, ApiError> {
    let email = normalize_email(&payload.email)?;
    let row = sqlx::query_as::<_, LoginAdminRow>(
        r#"
        select users.id as user_id,
               users.password_hash,
               users.email,
               users.email_verified_at,
               case when ara.role = 'sub_admin' then 'admin' else ara.role end as role,
               ara.capabilities,
               factor.id as totp_factor_id,
               factor.secret_ciphertext,
               factor.secret_nonce,
               factor.last_used_step
        from users
        join admin_role_assignments ara on ara.user_id = users.id
        left join lateral (
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
    .bind(&email)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::Unauthorized)?;

    let password_hash = row.password_hash.as_deref().ok_or(ApiError::Unauthorized)?;
    if !verify_password(password_hash, &payload.password) {
        return Err(ApiError::Unauthorized);
    }

    let actor = admin_context(row.user_id, &row.role, &row.capabilities)?;
    if row.email_verified_at.is_none() {
        audit_login_gate(&state, &actor, "admin_auth_email_verification_required", "Delegated admin email verification is required.").await?;
        return Err(ApiError::BadRequest(
            "admin email verification is required before delegated login".to_string(),
        ));
    }

    let (factor_id, ciphertext, nonce) = match (row.totp_factor_id, row.secret_ciphertext, row.secret_nonce) {
        (Some(factor_id), Some(ciphertext), Some(nonce)) => (factor_id, ciphertext, nonce),
        _ => {
            audit_login_gate(&state, &actor, "admin_auth_mfa_setup_required", "Delegated admin TOTP setup is required.").await?;
            return Err(ApiError::BadRequest(
                "admin 2FA setup is required before delegated login".to_string(),
            ));
        }
    };

    let Some(totp_code) = payload.totp_code.as_deref() else {
        audit_login_gate(&state, &actor, "admin_auth_mfa_required", "Delegated admin TOTP code is required.").await?;
        return Err(ApiError::BadRequest(
            "admin 2FA code is required before delegated login".to_string(),
        ));
    };
    let secret = decrypt_totp_secret(&ciphertext, &nonce)?;
    let used_step = verify_totp_code(&secret, totp_code, row.last_used_step)?;
    sqlx::query(
        r#"
        update admin_totp_factors
        set last_used_step = $2, updated_at = now()
        where id = $1 and revoked_at is null
        "#,
    )
    .bind(factor_id)
    .bind(used_step)
    .execute(&state.db)
    .await?;

    let (token, expires_at) = create_admin_session(&state, &actor).await?;

    admin_auth::audit(
        &state,
        &actor,
        "admin_auth_login",
        "admin_session",
        actor.actor_user_id,
        None,
        &actor.capabilities,
        "Delegated admin session was created after email verification and TOTP.",
        json!({"role": actor.role, "expires_at": expires_at, "email_verified": true, "totp_factor_id": factor_id}),
    )
    .await?;

    let mut response = (StatusCode::OK, success(AdminAuthData { actor, expires_at })).into_response();
    set_cookie_header(&mut response, admin_session_cookie(&state, &token));
    Ok(response)
}

async fn request_email_verification(
    State(state): State<AppState>,
    Json(payload): Json<EmailRequest>,
) -> Result<Json<AdminEnvelope<EmailRequestData>>, ApiError> {
    let (actor, row) = load_admin_password_actor(&state, &payload.email, &payload.password).await?;
    let token = new_token();
    let token_hash = hash_token(&token);
    let expires_at = Utc::now() + Duration::minutes(EMAIL_TOKEN_MINUTES);
    sqlx::query(
        r#"
        insert into admin_email_verification_tokens (
          id, user_id, token_hash, expires_at, metadata
        ) values ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(row.user_id)
    .bind(token_hash)
    .bind(expires_at)
    .bind(json!({"source": "admin_auth_api"}))
    .execute(&state.db)
    .await?;

    admin_auth::audit(
        &state,
        &actor,
        "admin_email_verification_requested",
        "admin_email_verification_token",
        Some(row.user_id),
        None,
        &actor.capabilities,
        "Delegated admin email verification token was created.",
        json!({"email": row.email, "expires_at": expires_at}),
    )
    .await?;

    let return_token = std::env::var("SPARK_ADMIN_AUTH_RETURN_BOOTSTRAP_TOKENS")
        .ok()
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    Ok(success(EmailRequestData {
        email: row.email,
        expires_at,
        delivery_mode: if return_token { "manual_bootstrap" } else { "email_delivery_pending" },
        manual_token: if return_token { Some(token) } else { None },
    }))
}

async fn confirm_email_verification(
    State(state): State<AppState>,
    Json(payload): Json<EmailConfirmRequest>,
) -> Result<Json<AdminEnvelope<EmailConfirmData>>, ApiError> {
    let (actor, row) = load_admin_password_actor(&state, &payload.email, &payload.password).await?;
    let token = clean_token(&payload.token)?;
    let token_hash = hash_token(&token);
    let updated = sqlx::query_scalar::<_, Option<Uuid>>(
        r#"
        update admin_email_verification_tokens
        set used_at = now(), attempt_count = attempt_count + 1
        where user_id = $1
          and token_hash = $2
          and used_at is null
          and expires_at > now()
        returning id
        "#,
    )
    .bind(row.user_id)
    .bind(token_hash)
    .fetch_optional(&state.db)
    .await?
    .flatten();

    if updated.is_none() {
        return Err(ApiError::Unauthorized);
    }

    let verified_at = Utc::now();
    sqlx::query("update users set email_verified_at = coalesce(email_verified_at, $2) where id = $1")
        .bind(row.user_id)
        .bind(verified_at)
        .execute(&state.db)
        .await?;

    admin_auth::audit(
        &state,
        &actor,
        "admin_email_verified",
        "user",
        Some(row.user_id),
        Some(row.user_id),
        &actor.capabilities,
        "Delegated admin email was verified.",
        json!({"email": row.email}),
    )
    .await?;

    Ok(success(EmailConfirmData { email: row.email, verified_at }))
}

async fn setup_totp(
    State(state): State<AppState>,
    Json(payload): Json<TotpSetupRequest>,
) -> Result<Json<AdminEnvelope<TotpSetupData>>, ApiError> {
    let (actor, row) = load_admin_password_actor(&state, &payload.email, &payload.password).await?;
    if row.email_verified_at.is_none() {
        return Err(ApiError::BadRequest(
            "admin email verification is required before TOTP setup".to_string(),
        ));
    }

    let already_enabled = sqlx::query_scalar::<_, bool>(
        r#"
        select exists(
          select 1 from admin_totp_factors
          where user_id = $1 and enabled_at is not null and revoked_at is null
        )
        "#,
    )
    .bind(row.user_id)
    .fetch_one(&state.db)
    .await?;
    if already_enabled {
        return Err(ApiError::Conflict("admin TOTP factor is already enabled".to_string()));
    }

    let mut secret = [0_u8; 20];
    OsRng.fill_bytes(&mut secret);
    let manual_secret = BASE32_NOPAD.encode(&secret);
    let (ciphertext, nonce) = encrypt_totp_secret(&secret)?;
    let factor_id = Uuid::new_v4();
    sqlx::query(
        r#"
        insert into admin_totp_factors (
          id, user_id, label, secret_ciphertext, secret_nonce, metadata
        ) values ($1, $2, 'Karyra Spark Admin', $3, $4, $5)
        "#,
    )
    .bind(factor_id)
    .bind(row.user_id)
    .bind(ciphertext)
    .bind(nonce)
    .bind(json!({"source": "admin_auth_api", "status": "pending_confirmation"}))
    .execute(&state.db)
    .await?;

    admin_auth::audit(
        &state,
        &actor,
        "admin_totp_setup_started",
        "admin_totp_factor",
        Some(row.user_id),
        Some(factor_id),
        &actor.capabilities,
        "Delegated admin TOTP setup was started.",
        json!({"email": row.email}),
    )
    .await?;

    let account_name = row.email;
    let otpauth_uri = format!(
        "otpauth://totp/Karyra%20Spark:{}?secret={}&issuer=Karyra%20Spark&algorithm=SHA1&digits=6&period=30",
        url_component(&account_name),
        manual_secret
    );

    Ok(success(TotpSetupData {
        factor_id,
        issuer: "Karyra Spark",
        account_name,
        otpauth_uri,
        manual_secret,
    }))
}

async fn confirm_totp(
    State(state): State<AppState>,
    Json(payload): Json<TotpConfirmRequest>,
) -> Result<Json<AdminEnvelope<TotpConfirmData>>, ApiError> {
    let (actor, row) = load_admin_password_actor(&state, &payload.email, &payload.password).await?;
    if row.email_verified_at.is_none() {
        return Err(ApiError::BadRequest(
            "admin email verification is required before TOTP confirmation".to_string(),
        ));
    }
    let factor = sqlx::query_as::<_, TotpFactorRow>(
        r#"
        select id, secret_ciphertext, secret_nonce, last_used_step
        from admin_totp_factors
        where id = $1
          and user_id = $2
          and enabled_at is null
          and revoked_at is null
        "#,
    )
    .bind(payload.factor_id)
    .bind(row.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::Unauthorized)?;

    let secret = decrypt_totp_secret(&factor.secret_ciphertext, &factor.secret_nonce)?;
    let used_step = verify_totp_code(&secret, &payload.code, factor.last_used_step)?;
    let enabled_at = Utc::now();
    sqlx::query(
        r#"
        update admin_totp_factors
        set enabled_at = $2,
            verified_at = $2,
            last_used_step = $3,
            updated_at = now(),
            metadata = metadata || $4::jsonb
        where id = $1
        "#,
    )
    .bind(factor.id)
    .bind(enabled_at)
    .bind(used_step)
    .bind(json!({"status": "enabled"}))
    .execute(&state.db)
    .await?;

    admin_auth::audit(
        &state,
        &actor,
        "admin_totp_enabled",
        "admin_totp_factor",
        Some(row.user_id),
        Some(factor.id),
        &actor.capabilities,
        "Delegated admin TOTP factor was enabled.",
        json!({"email": row.email}),
    )
    .await?;

    Ok(success(TotpConfirmData { factor_id: factor.id, enabled_at }))
}

async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminEnvelope<AdminAuthData>>, ApiError> {
    let actor = admin_auth::authorize_admin_actor(&state, &headers).await?;
    Ok(success(AdminAuthData {
        actor,
        expires_at: Utc::now() + Duration::hours(ADMIN_SESSION_HOURS),
    }))
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<Response, ApiError> {
    if let Ok(token) = read_session_cookie(&headers, admin_auth::ADMIN_SESSION_COOKIE_NAME) {
        let token_hash = hash_token(&token);
        sqlx::query(
            r#"
            update admin_sessions
            set revoked_at = now(), last_seen_at = now()
            where token_hash = $1 and revoked_at is null
            "#,
        )
        .bind(token_hash)
        .execute(&state.db)
        .await?;
    }

    let mut response = StatusCode::NO_CONTENT.into_response();
    set_cookie_header(&mut response, expired_admin_session_cookie(&state));
    Ok(response)
}

async fn load_admin_password_actor(
    state: &AppState,
    email: &str,
    password: &str,
) -> Result<(admin_auth::AdminContext, AdminPasswordRow), ApiError> {
    let email = normalize_email(email)?;
    let row = sqlx::query_as::<_, AdminPasswordRow>(
        r#"
        select users.id as user_id,
               users.password_hash,
               users.email,
               users.email_verified_at,
               case when ara.role = 'sub_admin' then 'admin' else ara.role end as role,
               ara.capabilities
        from users
        join admin_role_assignments ara on ara.user_id = users.id
        where lower(users.email) = lower($1)
          and users.status = 'active'
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
    .bind(&email)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::Unauthorized)?;

    let password_hash = row.password_hash.as_deref().ok_or(ApiError::Unauthorized)?;
    if !verify_password(password_hash, password) {
        return Err(ApiError::Unauthorized);
    }
    let actor = admin_context(row.user_id, &row.role, &row.capabilities)?;
    Ok((actor, row))
}

fn admin_context(user_id: Uuid, role: &str, capabilities: &[String]) -> Result<admin_auth::AdminContext, ApiError> {
    let role = admin_auth::canonical_role(role);
    if role != "admin" && role != "moderator" {
        return Err(ApiError::Unauthorized);
    }
    Ok(admin_auth::AdminContext {
        actor_kind: role.clone(),
        actor_user_id: Some(user_id),
        role: role.clone(),
        capabilities: admin_auth::sanitize_capabilities_for_role(&role, capabilities),
    })
}

async fn create_admin_session(
    state: &AppState,
    actor: &admin_auth::AdminContext,
) -> Result<(String, DateTime<Utc>), ApiError> {
    let token = new_token();
    let token_hash = hash_token(&token);
    let expires_at = Utc::now() + Duration::hours(ADMIN_SESSION_HOURS);

    sqlx::query(
        r#"
        insert into admin_sessions (
          id, user_id, token_hash, role, capabilities, expires_at, last_seen_at, metadata
        ) values ($1, $2, $3, $4, $5, $6, now(), $7)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(actor.actor_user_id.ok_or(ApiError::Unauthorized)?)
    .bind(token_hash)
    .bind(&actor.role)
    .bind(actor.capabilities.clone())
    .bind(expires_at)
    .bind(json!({"source": "admin_auth_api"}))
    .execute(&state.db)
    .await?;

    Ok((token, expires_at))
}

async fn audit_login_gate(
    state: &AppState,
    actor: &admin_auth::AdminContext,
    action: &str,
    summary: &str,
) -> Result<(), ApiError> {
    admin_auth::audit(
        state,
        actor,
        action,
        "admin_auth_gate",
        actor.actor_user_id,
        None,
        &actor.capabilities,
        summary,
        json!({"role": actor.role}),
    )
    .await
}

fn normalize_email(input: &str) -> Result<String, ApiError> {
    let email = input.trim().to_ascii_lowercase();
    let valid = email.len() <= 254
        && email.contains('@')
        && !email.starts_with('@')
        && !email.ends_with('@')
        && !email.contains(' ');
    if valid {
        Ok(email)
    } else {
        Err(ApiError::BadRequest("a valid email is required".to_string()))
    }
}

fn clean_token(input: &str) -> Result<String, ApiError> {
    let token = input.trim();
    if token.len() < 32 || token.len() > 160 || !token.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(ApiError::BadRequest("verification token is invalid".to_string()));
    }
    Ok(token.to_string())
}

fn clean_totp_code(input: &str) -> Result<String, ApiError> {
    let code: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        return Err(ApiError::BadRequest("TOTP code must be 6 digits".to_string()));
    }
    Ok(code)
}

fn verify_password(password_hash: &str, password: &str) -> bool {
    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(hash) => hash,
        Err(error) => {
            tracing::warn!(?error, "stored delegated admin password hash is invalid");
            return false;
        }
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

fn new_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
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
            tracing::error!(?error, "failed to encrypt admin TOTP secret");
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
            tracing::warn!(?error, "failed to decrypt admin TOTP secret");
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
    let mut mac = HmacSha1::new_from_slice(secret).map_err(|_| ApiError::Internal)?;
    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let offset = (digest[19] & 0x0f) as usize;
    let binary = ((digest[offset] as u32 & 0x7f) << 24)
        | ((digest[offset + 1] as u32) << 16)
        | ((digest[offset + 2] as u32) << 8)
        | (digest[offset + 3] as u32);
    Ok(format!("{:06}", binary % 1_000_000))
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

fn admin_session_cookie(state: &AppState, token: &str) -> String {
    let max_age = ADMIN_SESSION_HOURS * 60 * 60;
    let mut cookie = format!(
        "{}={}; Path=/; Max-Age={}; HttpOnly; SameSite=Lax",
        admin_auth::ADMIN_SESSION_COOKIE_NAME,
        token,
        max_age
    );
    if state.config.cookie_secure {
        cookie.push_str("; Secure");
    }
    cookie
}

fn expired_admin_session_cookie(state: &AppState) -> String {
    let mut cookie = format!(
        "{}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax",
        admin_auth::ADMIN_SESSION_COOKIE_NAME
    );
    if state.config.cookie_secure {
        cookie.push_str("; Secure");
    }
    cookie
}

fn set_cookie_header(response: &mut Response, cookie: String) {
    match HeaderValue::from_str(&cookie) {
        Ok(value) => {
            response.headers_mut().insert(header::SET_COOKIE, value);
        }
        Err(error) => tracing::error!(?error, "failed to create admin Set-Cookie header"),
    }
}
