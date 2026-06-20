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
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{admin_auth, auth::session::read_session_cookie, error::ApiError, state::AppState};

const ADMIN_SESSION_HOURS: i64 = 8;

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
        phase: "delegated-admin-email-mfa-foundation",
        routes: vec![
            "POST /api/admin/auth/login",
            "GET /api/admin/auth/me",
            "POST /api/admin/auth/logout",
        ],
        cookie_name: admin_auth::ADMIN_SESSION_COOKIE_NAME,
        auth_model: "superadmin remains root-token/session based; admin/moderator use dedicated admin session cookie",
        security_gates: vec![
            "delegated admin email must be verified",
            "delegated admin TOTP factor must be enabled",
            "no delegated admin session is created before both gates pass",
        ],
    })
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, FromRow)]
struct LoginAdminRow {
    user_id: Uuid,
    password_hash: Option<String>,
    email_verified_at: Option<DateTime<Utc>>,
    role: String,
    capabilities: Vec<String>,
    totp_enabled: bool,
}

#[derive(Serialize)]
struct AdminAuthData {
    actor: admin_auth::AdminContext,
    expires_at: DateTime<Utc>,
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
               users.email_verified_at,
               case when ara.role = 'sub_admin' then 'admin' else ara.role end as role,
               ara.capabilities,
               exists(
                 select 1
                 from admin_totp_factors factor
                 where factor.user_id = users.id
                   and factor.enabled_at is not null
                   and factor.revoked_at is null
               ) as totp_enabled
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
    if !verify_password(password_hash, &payload.password) {
        return Err(ApiError::Unauthorized);
    }

    let role = admin_auth::canonical_role(&row.role);
    if role != "admin" && role != "moderator" {
        return Err(ApiError::Unauthorized);
    }
    let capabilities = admin_auth::sanitize_capabilities_for_role(&role, &row.capabilities);
    let actor = admin_auth::AdminContext {
        actor_kind: role.clone(),
        actor_user_id: Some(row.user_id),
        role,
        capabilities,
    };

    if row.email_verified_at.is_none() {
        audit_login_gate(&state, &actor, "admin_auth_email_verification_required", "Delegated admin email verification is required.").await?;
        return Err(ApiError::BadRequest(
            "admin email verification is required before delegated login".to_string(),
        ));
    }

    if !row.totp_enabled {
        audit_login_gate(&state, &actor, "admin_auth_mfa_setup_required", "Delegated admin TOTP setup is required.").await?;
        return Err(ApiError::BadRequest(
            "admin 2FA setup is required before delegated login".to_string(),
        ));
    }

    let (token, expires_at) = create_admin_session(&state, &actor).await?;

    admin_auth::audit(
        &state,
        &actor,
        "admin_auth_login",
        "admin_session",
        actor.actor_user_id,
        None,
        &actor.capabilities,
        "Delegated admin session was created.",
        json!({"role": actor.role, "expires_at": expires_at, "email_verified": true, "totp_enabled": true}),
    )
    .await?;

    let mut response = (StatusCode::OK, success(AdminAuthData { actor, expires_at })).into_response();
    set_cookie_header(&mut response, admin_session_cookie(&state, &token));
    Ok(response)
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

async fn create_admin_session(
    state: &AppState,
    actor: &admin_auth::AdminContext,
) -> Result<(String, DateTime<Utc>), ApiError> {
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
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

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
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
