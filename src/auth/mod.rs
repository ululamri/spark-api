pub mod session;

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

const MIN_PASSWORD_LEN: usize = 8;
const MAX_PASSWORD_LEN: usize = 128;
const LOGIN_EMAIL_MAX_ATTEMPTS: i32 = 8;
const LOGIN_CLIENT_MAX_ATTEMPTS: i32 = 40;
const REGISTER_EMAIL_MAX_ATTEMPTS: i32 = 3;
const REGISTER_CLIENT_MAX_ATTEMPTS: i32 = 20;
const AUTH_WINDOW_SECONDS: i32 = 15 * 60;
const REGISTER_EMAIL_WINDOW_SECONDS: i32 = 60 * 60;

#[derive(Serialize)]
struct ScopeResponse {
    module: &'static str,
    phase: &'static str,
    implemented_now: Vec<&'static str>,
    next_backend_steps: Vec<&'static str>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
struct AuthUser {
    id: Uuid,
    email: String,
    display_name: String,
    handle: Option<String>,
}

#[derive(Debug, Serialize)]
struct AuthResponse {
    user: AuthUser,
}

#[derive(Debug, FromRow)]
struct UserIdRow {
    id: Uuid,
}

#[derive(Debug, FromRow)]
struct LoginUserRow {
    id: Uuid,
    password_hash: Option<String>,
}

#[derive(Debug, FromRow)]
struct AuthUserRow {
    id: Uuid,
    email: String,
    display_name: String,
    handle: Option<String>,
}

#[derive(Debug, FromRow)]
struct RateLimitRow {
    attempt_count: i32,
    window_expires_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/scope", get(scope))
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/me", get(me))
        .route("/logout", post(logout))
}

async fn scope() -> Json<ScopeResponse> {
    Json(ScopeResponse {
        module: module_path!(),
        phase: "auth-registration-readiness-hardening",
        implemented_now: vec![
            "user-registration",
            "password-hashing-argon2",
            "session-token-hash-storage",
            "httponly-cookie-session",
            "current-user-endpoint",
            "logout-session-revocation",
            "email-and-client-bucket-rate-limiting",
        ],
        next_backend_steps: vec![
            "email-verification",
            "password-reset",
            "role-policy-review",
            "frontend-session-hydration-qa",
        ],
    })
}

async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RegisterRequest>,
) -> Result<Response, ApiError> {
    let email = normalize_email(&payload.email)?;
    validate_password(&payload.password)?;
    enforce_auth_rate_limit(
        &state,
        "auth_register_email",
        &format!("email:{email}"),
        REGISTER_EMAIL_MAX_ATTEMPTS,
        REGISTER_EMAIL_WINDOW_SECONDS,
    )
    .await?;
    if let Some(identity) = client_rate_identity(&headers) {
        enforce_auth_rate_limit(
            &state,
            "auth_register_client",
            &identity,
            REGISTER_CLIENT_MAX_ATTEMPTS,
            AUTH_WINDOW_SECONDS,
        )
        .await?;
    }

    let exists: Option<(Uuid,)> =
        sqlx::query_as("select id from users where lower(email) = lower($1)")
            .bind(&email)
            .fetch_optional(&state.db)
            .await?;

    if exists.is_some() {
        return Err(ApiError::Conflict(
            "email is already registered".to_string(),
        ));
    }

    let password_hash = hash_password(&payload.password)?;
    let user = sqlx::query_as::<_, UserIdRow>(
        r#"
        insert into users (email, password_hash)
        values ($1, $2)
        returning id
        "#,
    )
    .bind(&email)
    .bind(&password_hash)
    .fetch_one(&state.db)
    .await?;

    let display_name = clean_display_name(payload.display_name.as_deref(), &email);
    sqlx::query(
        r#"
        insert into profiles (user_id, display_name)
        values ($1, $2)
        on conflict (user_id) do update set
          display_name = excluded.display_name,
          updated_at = now()
        "#,
    )
    .bind(user.id)
    .bind(display_name)
    .execute(&state.db)
    .await?;

    let auth_user = fetch_auth_user(&state, user.id).await?;
    let session_token = create_session(&state, user.id).await?;
    auth_response(
        StatusCode::CREATED,
        &state,
        Some(session_token),
        AuthResponse { user: auth_user },
    )
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<Response, ApiError> {
    let email = normalize_email(&payload.email)?;
    enforce_auth_rate_limit(
        &state,
        "auth_login_email",
        &format!("email:{email}"),
        LOGIN_EMAIL_MAX_ATTEMPTS,
        AUTH_WINDOW_SECONDS,
    )
    .await?;
    if let Some(identity) = client_rate_identity(&headers) {
        enforce_auth_rate_limit(
            &state,
            "auth_login_client",
            &identity,
            LOGIN_CLIENT_MAX_ATTEMPTS,
            AUTH_WINDOW_SECONDS,
        )
        .await?;
    }

    let user = sqlx::query_as::<_, LoginUserRow>(
        r#"
        select id, password_hash
        from users
        where lower(email) = lower($1) and status = 'active'
        "#,
    )
    .bind(&email)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::Unauthorized)?;

    let password_hash = user
        .password_hash
        .as_deref()
        .ok_or(ApiError::Unauthorized)?;
    if !verify_password(password_hash, &payload.password) {
        return Err(ApiError::Unauthorized);
    }

    let auth_user = fetch_auth_user(&state, user.id).await?;
    let session_token = create_session(&state, user.id).await?;
    auth_response(
        StatusCode::OK,
        &state,
        Some(session_token),
        AuthResponse { user: auth_user },
    )
}

async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AuthResponse>, ApiError> {
    let token = read_session_cookie(&headers, &state.config.session_cookie_name)?;
    let token_hash = hash_session_token(&token);
    let row = sqlx::query_as::<_, AuthUserRow>(
        r#"
        select users.id,
               users.email,
               coalesce(profiles.display_name, '') as display_name,
               profiles.handle
        from sessions
        join users on users.id = sessions.user_id
        left join profiles on profiles.user_id = users.id
        where sessions.token_hash = $1
          and sessions.expires_at > now()
          and sessions.revoked_at is null
          and users.status = 'active'
        "#,
    )
    .bind(token_hash)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::Unauthorized)?;

    Ok(Json(AuthResponse {
        user: AuthUser::from(row),
    }))
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<Response, ApiError> {
    if let Ok(token) = read_session_cookie(&headers, &state.config.session_cookie_name) {
        let token_hash = hash_session_token(&token);
        sqlx::query(
            r#"
            update sessions
            set revoked_at = now(), last_seen_at = now()
            where token_hash = $1 and revoked_at is null
            "#,
        )
        .bind(token_hash)
        .execute(&state.db)
        .await?;
    }

    let mut response = StatusCode::NO_CONTENT.into_response();
    set_cookie_header(&mut response, expired_session_cookie(&state));
    Ok(response)
}

async fn fetch_auth_user(state: &AppState, user_id: Uuid) -> Result<AuthUser, ApiError> {
    let row = sqlx::query_as::<_, AuthUserRow>(
        r#"
        select users.id,
               users.email,
               coalesce(profiles.display_name, '') as display_name,
               profiles.handle
        from users
        left join profiles on profiles.user_id = users.id
        where users.id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(AuthUser::from(row))
}

async fn create_session(state: &AppState, user_id: Uuid) -> Result<String, ApiError> {
    let session_token = new_session_token();
    let token_hash = hash_session_token(&session_token);
    let expires_at = Utc::now() + Duration::days(state.config.session_ttl_days);

    sqlx::query(
        r#"
        insert into sessions (user_id, token_hash, expires_at, last_seen_at)
        values ($1, $2, $3, now())
        "#,
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .execute(&state.db)
    .await?;

    Ok(session_token)
}

async fn enforce_auth_rate_limit(
    state: &AppState,
    bucket: &str,
    key: &str,
    max_attempts: i32,
    window_seconds: i32,
) -> Result<(), ApiError> {
    let key_hash = hash_rate_key(key);
    let row = sqlx::query_as::<_, RateLimitRow>(
        r#"
        insert into auth_rate_limits (
          bucket, key_hash, attempt_count, window_expires_at, metadata
        ) values ($1, $2, 1, now() + ($3::int * interval '1 second'), $4)
        on conflict (bucket, key_hash)
        do update set
          attempt_count = case
            when auth_rate_limits.window_expires_at <= now() then 1
            else auth_rate_limits.attempt_count + 1
          end,
          window_expires_at = case
            when auth_rate_limits.window_expires_at <= now() then now() + ($3::int * interval '1 second')
            else auth_rate_limits.window_expires_at
          end,
          last_seen_at = now(),
          metadata = excluded.metadata
        returning attempt_count, window_expires_at
        "#,
    )
    .bind(bucket)
    .bind(key_hash)
    .bind(window_seconds)
    .bind(json!({"window_seconds": window_seconds, "max_attempts": max_attempts}))
    .fetch_one(&state.db)
    .await?;

    if row.attempt_count > max_attempts {
        return Err(ApiError::RateLimited(format!(
            "too many auth attempts; try again after {}",
            row.window_expires_at.to_rfc3339()
        )));
    }

    Ok(())
}

fn auth_response(
    status: StatusCode,
    state: &AppState,
    session_token: Option<String>,
    body: AuthResponse,
) -> Result<Response, ApiError> {
    let mut response = (status, Json(body)).into_response();

    if let Some(token) = session_token {
        set_cookie_header(&mut response, session_cookie(state, &token));
    }

    Ok(response)
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
        Err(ApiError::BadRequest(
            "a valid email is required".to_string(),
        ))
    }
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

fn clean_display_name(input: Option<&str>, email: &str) -> String {
    let fallback = email.split('@').next().unwrap_or("Spark Learner");
    let display_name = input.unwrap_or(fallback).trim();

    if display_name.is_empty() {
        fallback.chars().take(64).collect()
    } else {
        display_name.chars().take(64).collect()
    }
}

fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::encode_b64(Uuid::new_v4().as_bytes()).map_err(|error| {
        tracing::error!(?error, "failed to create password salt");
        ApiError::Internal
    })?;

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| {
            tracing::error!(?error, "failed to hash password");
            ApiError::Internal
        })
}

fn verify_password(password_hash: &str, password: &str) -> bool {
    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(hash) => hash,
        Err(error) => {
            tracing::warn!(?error, "stored password hash is invalid");
            return false;
        }
    };

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

fn new_session_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn hash_session_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn hash_rate_key(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn client_rate_identity(headers: &HeaderMap) -> Option<String> {
    for name in ["cf-connecting-ip", "x-real-ip", "x-forwarded-for"] {
        if let Some(value) = headers.get(name).and_then(|value| value.to_str().ok()) {
            let candidate = value
                .split(',')
                .next()
                .unwrap_or("")
                .trim()
                .chars()
                .take(96)
                .collect::<String>();
            if !candidate.is_empty() {
                return Some(format!("client:{candidate}"));
            }
        }
    }
    None
}

fn read_session_cookie(headers: &HeaderMap, cookie_name: &str) -> Result<String, ApiError> {
    let cookie_header = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;

    for part in cookie_header.split(';') {
        let trimmed = part.trim();
        if let Some((name, value)) = trimmed.split_once('=') {
            if name == cookie_name && !value.trim().is_empty() {
                return Ok(value.trim().to_string());
            }
        }
    }

    Err(ApiError::Unauthorized)
}

fn session_cookie(state: &AppState, token: &str) -> String {
    let max_age = state.config.session_ttl_days * 24 * 60 * 60;
    let mut cookie = format!(
        "{}={}; Path=/; Max-Age={}; HttpOnly; SameSite=Lax",
        state.config.session_cookie_name, token, max_age
    );

    if state.config.cookie_secure {
        cookie.push_str("; Secure");
    }

    cookie
}

fn expired_session_cookie(state: &AppState) -> String {
    let mut cookie = format!(
        "{}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax",
        state.config.session_cookie_name
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
        Err(error) => {
            tracing::error!(?error, "failed to create Set-Cookie header");
        }
    }
}

impl From<AuthUserRow> for AuthUser {
    fn from(row: AuthUserRow) -> Self {
        Self {
            id: row.id,
            email: row.email,
            display_name: row.display_name,
            handle: row.handle,
        }
    }
}
