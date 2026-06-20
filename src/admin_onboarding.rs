use std::convert::TryInto;

use aes_gcm::{
    aead::{rand_core::RngCore, Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{extract::State, routing::{get, post}, Json, Router};
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

use crate::{admin_auth, error::ApiError, state::AppState};

type HmacSha1 = Hmac<Sha1>;

const EMAIL_OTP_MINUTES: i64 = 20;
const EMAIL_PROOF_MINUTES: i64 = 60;
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
        .route("/invite/inspect", post(inspect_invite))
        .route("/invite/email/request", post(request_invite_email_otp))
        .route("/invite/email/confirm", post(confirm_invite_email_otp))
        .route("/invite/password", post(set_invite_password))
        .route("/invite/totp/setup", post(setup_invite_totp))
        .route("/invite/totp/confirm", post(confirm_invite_totp))
        .route("/invite/accept", post(accept_invite))
}

#[derive(Serialize)]
struct ScopeData {
    module: &'static str,
    phase: &'static str,
    routes: Vec<&'static str>,
    onboarding_order: Vec<&'static str>,
    security_gates: Vec<&'static str>,
}

async fn scope() -> Json<AdminEnvelope<ScopeData>> {
    success(ScopeData {
        module: module_path!(),
        phase: "invite-token-admin-onboarding",
        routes: vec![
            "POST /api/admin/onboarding/invite/inspect",
            "POST /api/admin/onboarding/invite/email/request",
            "POST /api/admin/onboarding/invite/email/confirm",
            "POST /api/admin/onboarding/invite/password",
            "POST /api/admin/onboarding/invite/totp/setup",
            "POST /api/admin/onboarding/invite/totp/confirm",
            "POST /api/admin/onboarding/invite/accept",
        ],
        onboarding_order: vec![
            "validate invite token",
            "email must match invitation",
            "verify email OTP",
            "set password",
            "set and confirm authenticator 2FA",
            "accept invitation to activate delegated role",
        ],
        security_gates: vec![
            "raw invite token is never stored",
            "OTP is hashed and single-use",
            "email proof token is short-lived",
            "role assignment is created only after password plus enabled TOTP",
            "admin assignment activation uses invitation role/capabilities only",
        ],
    })
}

#[derive(Debug, Deserialize)]
struct InviteTokenRequest {
    token: String,
}

#[derive(Debug, Deserialize)]
struct InviteEmailRequest {
    token: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct InviteEmailConfirmRequest {
    token: String,
    email: String,
    otp: String,
}

#[derive(Debug, Deserialize)]
struct InvitePasswordRequest {
    token: String,
    email: String,
    email_proof_token: String,
    password: String,
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InviteTotpSetupRequest {
    token: String,
    email: String,
    email_proof_token: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct InviteTotpConfirmRequest {
    token: String,
    email: String,
    email_proof_token: String,
    password: String,
    factor_id: Uuid,
    code: String,
}

#[derive(Debug, Deserialize)]
struct InviteAcceptRequest {
    token: String,
    email: String,
    email_proof_token: String,
    password: String,
}

#[derive(Debug, Clone, FromRow)]
struct InviteRow {
    id: Uuid,
    email: String,
    role: String,
    capabilities: Vec<String>,
    invited_by_actor_kind: String,
    invited_by_user_id: Option<Uuid>,
    expires_at: DateTime<Utc>,
    accepted_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
struct UserRow {
    id: Uuid,
    email: String,
    password_hash: Option<String>,
    email_verified_at: Option<DateTime<Utc>>,
    status: String,
}

#[derive(Debug, FromRow)]
struct UserStatusRow {
    id: Uuid,
    email: String,
    status: String,
}

#[derive(Debug, FromRow)]
struct TotpFactorRow {
    id: Uuid,
    secret_ciphertext: Vec<u8>,
    secret_nonce: Vec<u8>,
    last_used_step: Option<i64>,
}

#[derive(Serialize)]
struct InviteInspectData {
    invitation_id: Uuid,
    email: String,
    role: String,
    capabilities: Vec<String>,
    expires_at: DateTime<Utc>,
    status: String,
}

#[derive(Serialize)]
struct InviteEmailOtpData {
    email: String,
    expires_at: DateTime<Utc>,
    delivery_mode: &'static str,
    manual_otp: Option<String>,
}

#[derive(Serialize)]
struct InviteEmailProofData {
    email: String,
    verified_at: DateTime<Utc>,
    email_proof_token: String,
    proof_expires_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct InvitePasswordData {
    user_id: Uuid,
    email: String,
    password_set_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct InviteTotpSetupData {
    factor_id: Uuid,
    issuer: &'static str,
    account_name: String,
    otpauth_uri: String,
    manual_secret: String,
}

#[derive(Serialize)]
struct InviteTotpConfirmData {
    factor_id: Uuid,
    enabled_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct InviteAcceptData {
    user_id: Uuid,
    email: String,
    role: String,
    capabilities: Vec<String>,
    accepted_at: DateTime<Utc>,
}

async fn inspect_invite(
    State(state): State<AppState>,
    Json(payload): Json<InviteTokenRequest>,
) -> Result<Json<AdminEnvelope<InviteInspectData>>, ApiError> {
    let invitation = load_pending_invitation(&state, &payload.token, None).await?;
    Ok(success(invite_inspect_data(invitation)))
}

async fn request_invite_email_otp(
    State(state): State<AppState>,
    Json(payload): Json<InviteEmailRequest>,
) -> Result<Json<AdminEnvelope<InviteEmailOtpData>>, ApiError> {
    let email = normalize_email(&payload.email)?;
    let invitation = load_pending_invitation(&state, &payload.token, Some(email.clone())).await?;
    let otp = new_otp();
    let otp_hash = hash_invite_otp(invitation.id, &otp);
    let expires_at = Utc::now() + Duration::minutes(EMAIL_OTP_MINUTES);

    sqlx::query(
        r#"
        insert into admin_invite_email_otps (
          id, invitation_id, email, otp_hash, expires_at, metadata
        ) values ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(invitation.id)
    .bind(&email)
    .bind(otp_hash)
    .bind(expires_at)
    .bind(json!({"source": "admin_invite_onboarding", "purpose": "email_ownership"}))
    .execute(&state.db)
    .await?;

    let return_token = return_bootstrap_tokens();
    Ok(success(InviteEmailOtpData {
        email,
        expires_at,
        delivery_mode: if return_token { "manual_bootstrap" } else { "email_delivery_pending" },
        manual_otp: if return_token { Some(otp) } else { None },
    }))
}

async fn confirm_invite_email_otp(
    State(state): State<AppState>,
    Json(payload): Json<InviteEmailConfirmRequest>,
) -> Result<Json<AdminEnvelope<InviteEmailProofData>>, ApiError> {
    let email = normalize_email(&payload.email)?;
    let invitation = load_pending_invitation(&state, &payload.token, Some(email.clone())).await?;
    let otp = clean_otp(&payload.otp)?;
    let otp_hash = hash_invite_otp(invitation.id, &otp);
    let proof_token = new_email_proof_token();
    let proof_hash = hash_token(&proof_token);
    let proof_expires_at = Utc::now() + Duration::minutes(EMAIL_PROOF_MINUTES);
    let verified_at = Utc::now();

    let updated = sqlx::query_scalar::<_, Option<Uuid>>(
        r#"
        update admin_invite_email_otps
        set consumed_at = $4,
            attempt_count = attempt_count + 1,
            metadata = metadata || $5::jsonb
        where invitation_id = $1
          and lower(email) = lower($2)
          and otp_hash = $3
          and consumed_at is null
          and expires_at > now()
        returning id
        "#,
    )
    .bind(invitation.id)
    .bind(&email)
    .bind(otp_hash)
    .bind(verified_at)
    .bind(json!({
        "email_verified_at": verified_at,
        "email_proof_token_hash": proof_hash,
        "email_proof_expires_at": proof_expires_at
    }))
    .fetch_optional(&state.db)
    .await?
    .flatten();

    if updated.is_none() {
        return Err(ApiError::Unauthorized);
    }

    Ok(success(InviteEmailProofData {
        email,
        verified_at,
        email_proof_token: proof_token,
        proof_expires_at,
    }))
}

async fn set_invite_password(
    State(state): State<AppState>,
    Json(payload): Json<InvitePasswordRequest>,
) -> Result<Json<AdminEnvelope<InvitePasswordData>>, ApiError> {
    let email = normalize_email(&payload.email)?;
    let invitation = load_pending_invitation(&state, &payload.token, Some(email.clone())).await?;
    ensure_email_proof(&state, invitation.id, &email, &payload.email_proof_token).await?;
    validate_password(&payload.password)?;
    let password_hash = hash_password(&payload.password)?;
    let password_set_at = Utc::now();
    let user = upsert_invited_user(
        &state,
        &email,
        &password_hash,
        password_set_at,
        payload.display_name.as_deref(),
    )
    .await?;

    let actor = onboarding_actor(user.id, &invitation.role, &invitation.capabilities);
    admin_auth::audit(
        &state,
        &actor,
        "admin_invite_password_set",
        "user",
        Some(user.id),
        Some(invitation.id),
        &invitation.capabilities,
        "Invited delegated admin password was set after email proof.",
        json!({"email": email, "role": invitation.role}),
    )
    .await?;

    Ok(success(InvitePasswordData {
        user_id: user.id,
        email: user.email,
        password_set_at,
    }))
}

async fn setup_invite_totp(
    State(state): State<AppState>,
    Json(payload): Json<InviteTotpSetupRequest>,
) -> Result<Json<AdminEnvelope<InviteTotpSetupData>>, ApiError> {
    let email = normalize_email(&payload.email)?;
    let invitation = load_pending_invitation(&state, &payload.token, Some(email.clone())).await?;
    ensure_email_proof(&state, invitation.id, &email, &payload.email_proof_token).await?;
    let user = load_invited_user_with_password(&state, &email, &payload.password).await?;
    ensure_user_ready_for_admin_onboarding(&user)?;

    let already_enabled = sqlx::query_scalar::<_, bool>(
        r#"
        select exists(
          select 1 from admin_totp_factors
          where user_id = $1 and enabled_at is not null and revoked_at is null
        )
        "#,
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;
    if already_enabled {
        return Err(ApiError::Conflict("admin TOTP factor is already enabled".to_string()));
    }

    sqlx::query(
        r#"
        update admin_totp_factors
        set revoked_at = now(), updated_at = now(), metadata = metadata || $2::jsonb
        where user_id = $1 and enabled_at is null and revoked_at is null
        "#,
    )
    .bind(user.id)
    .bind(json!({"revoked_by": "admin_invite_onboarding", "reason": "new pending factor requested"}))
    .execute(&state.db)
    .await?;

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
    .bind(user.id)
    .bind(ciphertext)
    .bind(nonce)
    .bind(json!({"source": "admin_invite_onboarding", "status": "pending_confirmation", "invitation_id": invitation.id}))
    .execute(&state.db)
    .await?;

    let actor = onboarding_actor(user.id, &invitation.role, &invitation.capabilities);
    admin_auth::audit(
        &state,
        &actor,
        "admin_invite_totp_setup_started",
        "admin_totp_factor",
        Some(user.id),
        Some(factor_id),
        &invitation.capabilities,
        "Invited delegated admin TOTP setup was started.",
        json!({"email": email, "role": invitation.role}),
    )
    .await?;

    let account_name = user.email;
    let otpauth_uri = format!(
        "otpauth://totp/Karyra%20Spark:{}?secret={}&issuer=Karyra%20Spark&algorithm=SHA1&digits=6&period=30",
        url_component(&account_name),
        manual_secret
    );

    Ok(success(InviteTotpSetupData {
        factor_id,
        issuer: "Karyra Spark",
        account_name,
        otpauth_uri,
        manual_secret,
    }))
}

async fn confirm_invite_totp(
    State(state): State<AppState>,
    Json(payload): Json<InviteTotpConfirmRequest>,
) -> Result<Json<AdminEnvelope<InviteTotpConfirmData>>, ApiError> {
    let email = normalize_email(&payload.email)?;
    let invitation = load_pending_invitation(&state, &payload.token, Some(email.clone())).await?;
    ensure_email_proof(&state, invitation.id, &email, &payload.email_proof_token).await?;
    let user = load_invited_user_with_password(&state, &email, &payload.password).await?;
    ensure_user_ready_for_admin_onboarding(&user)?;

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
    .bind(user.id)
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
    .bind(json!({"status": "enabled", "source": "admin_invite_onboarding"}))
    .execute(&state.db)
    .await?;

    let actor = onboarding_actor(user.id, &invitation.role, &invitation.capabilities);
    admin_auth::audit(
        &state,
        &actor,
        "admin_invite_totp_enabled",
        "admin_totp_factor",
        Some(user.id),
        Some(factor.id),
        &invitation.capabilities,
        "Invited delegated admin TOTP factor was enabled.",
        json!({"email": email, "role": invitation.role}),
    )
    .await?;

    Ok(success(InviteTotpConfirmData { factor_id: factor.id, enabled_at }))
}

async fn accept_invite(
    State(state): State<AppState>,
    Json(payload): Json<InviteAcceptRequest>,
) -> Result<Json<AdminEnvelope<InviteAcceptData>>, ApiError> {
    let email = normalize_email(&payload.email)?;
    let mut invitation = load_pending_invitation(&state, &payload.token, Some(email.clone())).await?;
    invitation.capabilities = admin_auth::sanitize_capabilities_for_role(&invitation.role, &invitation.capabilities);
    ensure_email_proof(&state, invitation.id, &email, &payload.email_proof_token).await?;
    let user = load_invited_user_with_password(&state, &email, &payload.password).await?;
    ensure_user_ready_for_admin_onboarding(&user)?;

    let factor_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        select id
        from admin_totp_factors
        where user_id = $1
          and enabled_at is not null
          and revoked_at is null
        order by enabled_at desc
        limit 1
        "#,
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("admin 2FA must be enabled before accepting invite".to_string()))?;

    let accepted_at = Utc::now();
    let assignment_id = Uuid::new_v4();
    sqlx::query(
        r#"
        insert into admin_role_assignments (
          id, user_id, role, capabilities, status, granted_by_user_id,
          granted_by_kind, reason, starts_at, expires_at, revoked_at, metadata
        ) values ($1, $2, $3, $4, 'active', $5, $6, $7, $8, null, null, $9)
        on conflict (user_id, role) where status = 'active' and revoked_at is null
        do update set
          capabilities = excluded.capabilities,
          reason = excluded.reason,
          expires_at = excluded.expires_at,
          granted_by_user_id = excluded.granted_by_user_id,
          granted_by_kind = excluded.granted_by_kind,
          metadata = excluded.metadata,
          updated_at = now()
        "#,
    )
    .bind(assignment_id)
    .bind(user.id)
    .bind(&invitation.role)
    .bind(invitation.capabilities.clone())
    .bind(invitation.invited_by_user_id)
    .bind(&invitation.invited_by_actor_kind)
    .bind("accepted invite-only onboarding")
    .bind(accepted_at)
    .bind(json!({
        "source": "admin_invite_onboarding",
        "invitation_id": invitation.id,
        "email": email
    }))
    .execute(&state.db)
    .await?;

    sqlx::query(
        r#"
        update admin_invitations
        set accepted_at = $2,
            accepted_by_user_id = $3,
            metadata = metadata || $4::jsonb
        where id = $1
          and accepted_at is null
          and revoked_at is null
          and expires_at > now()
        "#,
    )
    .bind(invitation.id)
    .bind(accepted_at)
    .bind(user.id)
    .bind(json!({"accepted_via": "admin_invite_onboarding", "totp_factor_id": factor_id}))
    .execute(&state.db)
    .await?;

    let actor = onboarding_actor(user.id, &invitation.role, &invitation.capabilities);
    admin_auth::audit(
        &state,
        &actor,
        "admin_invitation_accept",
        "admin_invitation",
        Some(user.id),
        Some(invitation.id),
        &invitation.capabilities,
        "Admin team invitation was accepted and role was activated.",
        json!({"email": email, "role": invitation.role, "assignment_id": assignment_id}),
    )
    .await?;

    Ok(success(InviteAcceptData {
        user_id: user.id,
        email: user.email,
        role: invitation.role,
        capabilities: invitation.capabilities,
        accepted_at,
    }))
}

fn invite_inspect_data(mut invitation: InviteRow) -> InviteInspectData {
    invitation.capabilities = admin_auth::sanitize_capabilities_for_role(&invitation.role, &invitation.capabilities);
    InviteInspectData {
        invitation_id: invitation.id,
        email: invitation.email,
        role: invitation.role,
        capabilities: invitation.capabilities,
        expires_at: invitation.expires_at,
        status: if invitation.accepted_at.is_some() {
            "accepted".to_string()
        } else if invitation.revoked_at.is_some() {
            "revoked".to_string()
        } else {
            "pending".to_string()
        },
    }
}

async fn load_pending_invitation(
    state: &AppState,
    token: &str,
    email: Option<String>,
) -> Result<InviteRow, ApiError> {
    let token = clean_invite_token(token)?;
    let token_hash = hash_token(&token);
    let mut invitation = sqlx::query_as::<_, InviteRow>(
        r#"
        select id,
               email,
               role,
               capabilities,
               invited_by_actor_kind,
               invited_by_user_id,
               expires_at,
               accepted_at,
               revoked_at
        from admin_invitations
        where token_hash = $1
          and ($2::text is null or lower(email) = lower($2))
          and accepted_at is null
          and revoked_at is null
          and expires_at > now()
        "#,
    )
    .bind(token_hash)
    .bind(email.as_deref())
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::Unauthorized)?;

    invitation.role = admin_auth::normalize_role(&invitation.role)?;
    invitation.capabilities = admin_auth::sanitize_capabilities_for_role(&invitation.role, &invitation.capabilities);
    Ok(invitation)
}

async fn ensure_email_proof(
    state: &AppState,
    invitation_id: Uuid,
    email: &str,
    proof_token: &str,
) -> Result<(), ApiError> {
    let proof_token = clean_email_proof_token(proof_token)?;
    let proof_hash = hash_token(&proof_token);
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        select exists(
          select 1
          from admin_invite_email_otps
          where invitation_id = $1
            and lower(email) = lower($2)
            and consumed_at is not null
            and metadata->>'email_proof_token_hash' = $3
            and (metadata->>'email_proof_expires_at')::timestamptz > now()
        )
        "#,
    )
    .bind(invitation_id)
    .bind(email)
    .bind(proof_hash)
    .fetch_one(&state.db)
    .await?;

    if exists {
        Ok(())
    } else {
        Err(ApiError::Unauthorized)
    }
}

async fn upsert_invited_user(
    state: &AppState,
    email: &str,
    password_hash: &str,
    verified_at: DateTime<Utc>,
    display_name: Option<&str>,
) -> Result<UserStatusRow, ApiError> {
    let existing = sqlx::query_as::<_, UserStatusRow>(
        "select id, email, status from users where lower(email) = lower($1)",
    )
    .bind(email)
    .fetch_optional(&state.db)
    .await?;

    let user = if let Some(user) = existing {
        if user.status != "active" {
            return Err(ApiError::Conflict("invited account is not active".to_string()));
        }
        sqlx::query_as::<_, UserStatusRow>(
            r#"
            update users
            set password_hash = $2,
                email_verified_at = coalesce(email_verified_at, $3)
            where id = $1
            returning id, email, status
            "#,
        )
        .bind(user.id)
        .bind(password_hash)
        .bind(verified_at)
        .fetch_one(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, UserStatusRow>(
            r#"
            insert into users (email, password_hash, email_verified_at)
            values ($1, $2, $3)
            returning id, email, status
            "#,
        )
        .bind(email)
        .bind(password_hash)
        .bind(verified_at)
        .fetch_one(&state.db)
        .await?
    };

    let name = clean_display_name(display_name, &user.email);
    sqlx::query(
        r#"
        insert into profiles (user_id, display_name)
        values ($1, $2)
        on conflict (user_id) do nothing
        "#,
    )
    .bind(user.id)
    .bind(name)
    .execute(&state.db)
    .await?;

    Ok(user)
}

async fn load_invited_user_with_password(
    state: &AppState,
    email: &str,
    password: &str,
) -> Result<UserRow, ApiError> {
    let user = sqlx::query_as::<_, UserRow>(
        r#"
        select id, email, password_hash, email_verified_at, status
        from users
        where lower(email) = lower($1)
        "#,
    )
    .bind(email)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::Unauthorized)?;

    if user.status != "active" {
        return Err(ApiError::Unauthorized);
    }
    let password_hash = user.password_hash.as_deref().ok_or(ApiError::Unauthorized)?;
    if !verify_password(password_hash, password) {
        return Err(ApiError::Unauthorized);
    }
    Ok(user)
}

fn ensure_user_ready_for_admin_onboarding(user: &UserRow) -> Result<(), ApiError> {
    if user.email_verified_at.is_none() {
        return Err(ApiError::BadRequest(
            "invite email verification is required before this step".to_string(),
        ));
    }
    Ok(())
}

fn onboarding_actor(user_id: Uuid, role: &str, capabilities: &[String]) -> admin_auth::AdminContext {
    let role = admin_auth::canonical_role(role);
    admin_auth::AdminContext {
        actor_kind: format!("{role}_invite_onboarding"),
        actor_user_id: Some(user_id),
        role: role.clone(),
        capabilities: admin_auth::sanitize_capabilities_for_role(&role, capabilities),
    }
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
    let fallback = email.split('@').next().unwrap_or("Spark Admin");
    let display_name = input.unwrap_or(fallback).trim();
    if display_name.is_empty() {
        fallback.chars().take(64).collect()
    } else {
        display_name.chars().take(64).collect()
    }
}

fn clean_invite_token(input: &str) -> Result<String, ApiError> {
    let token = input.trim();
    let valid = (40..=180).contains(&token.len())
        && token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if valid {
        Ok(token.to_string())
    } else {
        Err(ApiError::BadRequest("invite token is invalid".to_string()))
    }
}

fn clean_email_proof_token(input: &str) -> Result<String, ApiError> {
    let token = input.trim();
    let valid = (40..=180).contains(&token.len())
        && token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if valid {
        Ok(token.to_string())
    } else {
        Err(ApiError::BadRequest("email proof token is invalid".to_string()))
    }
}

fn clean_otp(input: &str) -> Result<String, ApiError> {
    let otp: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if otp.len() == 6 && otp.chars().all(|c| c.is_ascii_digit()) {
        Ok(otp)
    } else {
        Err(ApiError::BadRequest("OTP must be 6 digits".to_string()))
    }
}

fn clean_totp_code(input: &str) -> Result<String, ApiError> {
    let code: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        return Err(ApiError::BadRequest("TOTP code must be 6 digits".to_string()));
    }
    Ok(code)
}

fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::encode_b64(Uuid::new_v4().as_bytes()).map_err(|error| {
        tracing::error!(?error, "failed to create admin invite password salt");
        ApiError::Internal
    })?;

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| {
            tracing::error!(?error, "failed to hash invited admin password");
            ApiError::Internal
        })
}

fn verify_password(password_hash: &str, password: &str) -> bool {
    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(hash) => hash,
        Err(error) => {
            tracing::warn!(?error, "stored invited admin password hash is invalid");
            return false;
        }
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

fn new_otp() -> String {
    format!("{:06}", OsRng.next_u32() % 1_000_000)
}

fn new_email_proof_token() -> String {
    format!("adm_proof_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn hash_invite_otp(invitation_id: Uuid, otp: &str) -> String {
    hash_token(&format!("{invitation_id}:{otp}"))
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn return_bootstrap_tokens() -> bool {
    std::env::var("SPARK_ADMIN_INVITE_RETURN_BOOTSTRAP_TOKENS")
        .ok()
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
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
            tracing::error!(?error, "failed to encrypt invited admin TOTP secret");
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
            tracing::warn!(?error, "failed to decrypt invited admin TOTP secret");
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

fn url_component(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => vec![byte as char],
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}
