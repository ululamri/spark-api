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

const INVITE_TOKEN_DAYS: i64 = 7;

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
        .route("/actor", get(actor))
        .route("/members", get(members))
        .route("/members/:user_id/revoke", post(revoke_member))
        .route("/capabilities", get(capabilities))
        .route("/invitations", get(invitations).post(create_invitation))
        .route("/invitations/:invitation_id/revoke", post(revoke_invitation))
}

#[derive(Serialize)]
struct ScopeData {
    module: &'static str,
    phase: &'static str,
    roles: Vec<RoleInfo>,
    routes: Vec<&'static str>,
    auth_model: &'static str,
    invite_policy: Vec<&'static str>,
}

#[derive(Serialize)]
struct RoleInfo {
    role: &'static str,
    description: &'static str,
    capabilities: Vec<String>,
}

async fn scope(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminEnvelope<ScopeData>>, ApiError> {
    admin_auth::authorize_with_capability(&state, &headers, "audit_read").await?;
    Ok(success(ScopeData {
        module: module_path!(),
        phase: "invite-only-admin-team-model",
        roles: role_catalog(),
        routes: vec![
            "GET /api/admin/team/scope",
            "GET /api/admin/team/actor",
            "GET /api/admin/team/capabilities",
            "GET /api/admin/team/members",
            "POST /api/admin/team/members/:user_id/revoke",
            "GET /api/admin/team/invitations",
            "POST /api/admin/team/invitations",
            "POST /api/admin/team/invitations/:invitation_id/revoke",
        ],
        auth_model:
            "superadmin token bootstrap plus invite-only delegated admin/moderator onboarding",
        invite_policy: vec![
            "superadmin can invite admin or moderator",
            "admin can invite moderator only",
            "moderator cannot invite",
            "direct delegated role creation is disabled; role activation must complete invite onboarding",
            "invitation token is stored as a hash and is single-use",
        ],
    }))
}

async fn actor(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminEnvelope<admin_auth::AdminContext>>, ApiError> {
    let actor = admin_auth::authorize_admin_actor(&state, &headers).await?;
    Ok(success(actor))
}

async fn capabilities(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminEnvelope<Vec<RoleInfo>>>, ApiError> {
    admin_auth::authorize_with_capability(&state, &headers, "audit_read").await?;
    Ok(success(role_catalog()))
}

#[derive(Debug, Deserialize)]
struct MembersQuery {
    role: Option<String>,
    status: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Serialize)]
struct MembersData {
    items: Vec<admin_auth::AdminAssignmentRow>,
    total: i64,
    limit: i64,
    offset: i64,
}

async fn members(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<MembersQuery>,
) -> Result<Json<AdminEnvelope<MembersData>>, ApiError> {
    admin_auth::authorize_with_capability(&state, &headers, "audit_read").await?;
    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);
    let role = query
        .role
        .as_deref()
        .map(admin_auth::normalize_role)
        .transpose()?;
    let status = query
        .status
        .as_deref()
        .map(normalize_status)
        .transpose()?
        .unwrap_or_else(|| "active".to_string());

    let mut items = sqlx::query_as::<_, admin_auth::AdminAssignmentRow>(
        r#"
        with assignments as (
          select ara.id,
                 ara.user_id,
                 u.email,
                 coalesce(nullif(p.display_name, ''), split_part(u.email, '@', 1), 'Pengguna Spark') as display_name,
                 p.handle,
                 case when ara.role = 'sub_admin' then 'admin' else ara.role end as role,
                 ara.capabilities,
                 case
                   when ara.status = 'active'
                    and ara.revoked_at is null
                    and ara.starts_at <= now()
                    and (ara.expires_at is null or ara.expires_at > now()) then 'active'
                   when ara.status = 'active'
                    and ara.revoked_at is null
                    and ara.expires_at is not null
                    and ara.expires_at <= now() then 'expired'
                   when ara.status = 'revoked' or ara.revoked_at is not null then 'revoked'
                   else ara.status
                 end as status,
                 ara.reason,
                 ara.starts_at,
                 ara.expires_at,
                 ara.created_at,
                 ara.updated_at
          from admin_role_assignments ara
          join users u on u.id = ara.user_id
          left join profiles p on p.user_id = ara.user_id
          where ($1::text is null or ara.role = $1 or ($1 = 'admin' and ara.role = 'sub_admin'))
        )
        select * from assignments
        where status = $2
        order by updated_at desc, created_at desc
        limit $3 offset $4
        "#,
    )
    .bind(role.as_deref())
    .bind(&status)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    for item in &mut items {
        item.capabilities = admin_auth::sanitize_capabilities_for_role(&item.role, &item.capabilities);
    }

    let total = sqlx::query_scalar::<_, i64>(
        r#"
        with assignments as (
          select case
                   when ara.status = 'active'
                    and ara.revoked_at is null
                    and ara.starts_at <= now()
                    and (ara.expires_at is null or ara.expires_at > now()) then 'active'
                   when ara.status = 'active'
                    and ara.revoked_at is null
                    and ara.expires_at is not null
                    and ara.expires_at <= now() then 'expired'
                   when ara.status = 'revoked' or ara.revoked_at is not null then 'revoked'
                   else ara.status
                 end as status
          from admin_role_assignments ara
          where ($1::text is null or ara.role = $1 or ($1 = 'admin' and ara.role = 'sub_admin'))
        )
        select count(*) from assignments where status = $2
        "#,
    )
    .bind(role.as_deref())
    .bind(&status)
    .fetch_one(&state.db)
    .await?;

    Ok(success(MembersData {
        items,
        total,
        limit,
        offset,
    }))
}

#[derive(Debug, Deserialize)]
struct RevokeMemberRequest {
    role: String,
    reason: Option<String>,
}

#[derive(Serialize)]
struct MemberWriteData {
    assignment: admin_auth::AdminAssignmentRow,
}

async fn revoke_member(
    Path(user_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RevokeMemberRequest>,
) -> Result<Json<AdminEnvelope<MemberWriteData>>, ApiError> {
    let actor = admin_auth::authorize_admin_manage(&state, &headers).await?;
    let role = admin_auth::normalize_role(&payload.role)?;
    let reason = clean_reason(payload.reason.as_deref())?;

    let assignment = fetch_active_assignment(&state, user_id, &role).await?;
    sqlx::query(
        r#"
        update admin_role_assignments
        set status = 'revoked',
            revoked_at = now(),
            revoked_by_user_id = $3,
            reason = case when $4 = '' then reason else $4 end,
            updated_at = now()
        where user_id = $1
          and (role = $2 or ($2 = 'admin' and role = 'sub_admin'))
          and status = 'active'
          and revoked_at is null
          and starts_at <= now()
          and (expires_at is null or expires_at > now())
        "#,
    )
    .bind(user_id)
    .bind(&role)
    .bind(actor.actor_user_id)
    .bind(&reason)
    .execute(&state.db)
    .await?;

    admin_auth::audit(
        &state,
        &actor,
        "admin_role_revoke",
        "user",
        Some(user_id),
        Some(assignment.id),
        &assignment.capabilities,
        "Admin role assignment was revoked.",
        json!({"role": role, "reason": reason}),
    )
    .await?;

    Ok(success(MemberWriteData { assignment }))
}

#[derive(Debug, Deserialize)]
struct InvitationsQuery {
    role: Option<String>,
    status: Option<String>,
    email: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Serialize, FromRow)]
struct AdminInvitationRow {
    id: Uuid,
    email: String,
    role: String,
    capabilities: Vec<String>,
    invited_by_actor_kind: String,
    invited_by_user_id: Option<Uuid>,
    expires_at: DateTime<Utc>,
    accepted_at: Option<DateTime<Utc>>,
    accepted_by_user_id: Option<Uuid>,
    revoked_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    metadata: Value,
    status: String,
}

#[derive(Serialize)]
struct InvitationsData {
    items: Vec<AdminInvitationRow>,
    total: i64,
    limit: i64,
    offset: i64,
}

#[derive(Debug, Deserialize)]
struct CreateInvitationRequest {
    email: String,
    role: String,
    capabilities: Option<Vec<String>>,
    reason: Option<String>,
    expires_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
struct CreateInvitationData {
    invitation: AdminInvitationRow,
    delivery_mode: &'static str,
    manual_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RevokeInvitationRequest {
    reason: Option<String>,
}

#[derive(Serialize)]
struct RevokeInvitationData {
    invitation: AdminInvitationRow,
}

async fn invitations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<InvitationsQuery>,
) -> Result<Json<AdminEnvelope<InvitationsData>>, ApiError> {
    let actor = authorize_invitation_reader(&state, &headers).await?;
    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);
    let role = query
        .role
        .as_deref()
        .map(admin_auth::normalize_role)
        .transpose()?;
    let status = query
        .status
        .as_deref()
        .map(normalize_invitation_status)
        .transpose()?
        .unwrap_or_else(|| "pending".to_string());
    let email = query.email.as_deref().map(normalize_email).transpose()?;

    let mut items = sqlx::query_as::<_, AdminInvitationRow>(
        r#"
        with invitations as (
          select id,
                 email,
                 role,
                 capabilities,
                 invited_by_actor_kind,
                 invited_by_user_id,
                 expires_at,
                 accepted_at,
                 accepted_by_user_id,
                 revoked_at,
                 created_at,
                 metadata,
                 case
                   when accepted_at is not null then 'accepted'
                   when revoked_at is not null then 'revoked'
                   when expires_at <= now() then 'expired'
                   else 'pending'
                 end as status
          from admin_invitations
          where ($1::text is null or role = $1)
            and ($2::text is null or lower(email) = lower($2))
            and (
              $3::text = 'superadmin'
              or (role = 'moderator' and invited_by_user_id = $4)
            )
        )
        select * from invitations
        where ($5::text = 'all' or status = $5)
        order by created_at desc
        limit $6 offset $7
        "#,
    )
    .bind(role.as_deref())
    .bind(email.as_deref())
    .bind(&actor.role)
    .bind(actor.actor_user_id)
    .bind(&status)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    for invitation in &mut items {
        invitation.capabilities =
            admin_auth::sanitize_capabilities_for_role(&invitation.role, &invitation.capabilities);
    }

    let total = sqlx::query_scalar::<_, i64>(
        r#"
        with invitations as (
          select case
                   when accepted_at is not null then 'accepted'
                   when revoked_at is not null then 'revoked'
                   when expires_at <= now() then 'expired'
                   else 'pending'
                 end as status
          from admin_invitations
          where ($1::text is null or role = $1)
            and ($2::text is null or lower(email) = lower($2))
            and (
              $3::text = 'superadmin'
              or (role = 'moderator' and invited_by_user_id = $4)
            )
        )
        select count(*) from invitations
        where ($5::text = 'all' or status = $5)
        "#,
    )
    .bind(role.as_deref())
    .bind(email.as_deref())
    .bind(&actor.role)
    .bind(actor.actor_user_id)
    .bind(&status)
    .fetch_one(&state.db)
    .await?;

    Ok(success(InvitationsData {
        items,
        total,
        limit,
        offset,
    }))
}

async fn create_invitation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateInvitationRequest>,
) -> Result<Json<AdminEnvelope<CreateInvitationData>>, ApiError> {
    let actor = admin_auth::authorize_admin_actor(&state, &headers).await?;
    let role = admin_auth::normalize_role(&payload.role)?;
    ensure_can_invite_role(&actor, &role)?;
    let email = normalize_email(&payload.email)?;
    let requested_capabilities = payload.capabilities.unwrap_or_default();
    let capabilities = admin_auth::normalize_capabilities(&role, &requested_capabilities)?;
    let reason = clean_reason(payload.reason.as_deref())?;
    let expires_at = payload
        .expires_at
        .unwrap_or_else(|| Utc::now() + Duration::days(INVITE_TOKEN_DAYS));

    if expires_at <= Utc::now() {
        return Err(ApiError::BadRequest("expires_at must be in the future".to_string()));
    }

    let duplicate = sqlx::query_scalar::<_, bool>(
        r#"
        select exists(
          select 1
          from admin_invitations
          where lower(email) = lower($1)
            and role = $2
            and accepted_at is null
            and revoked_at is null
            and expires_at > now()
        )
        "#,
    )
    .bind(&email)
    .bind(&role)
    .fetch_one(&state.db)
    .await?;

    if duplicate {
        return Err(ApiError::Conflict(
            "an active invitation already exists for this email and role".to_string(),
        ));
    }

    let token = new_invite_token();
    let token_hash = hash_token(&token);
    let invitation_id = Uuid::new_v4();
    sqlx::query(
        r#"
        insert into admin_invitations (
          id, email, role, capabilities, token_hash, invited_by_actor_kind,
          invited_by_user_id, expires_at, metadata
        ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(invitation_id)
    .bind(&email)
    .bind(&role)
    .bind(capabilities.clone())
    .bind(token_hash)
    .bind(&actor.actor_kind)
    .bind(actor.actor_user_id)
    .bind(expires_at)
    .bind(json!({
        "reason": reason,
        "source": "admin_team_invitation_api",
        "policy": "invite_only_admin_model"
    }))
    .execute(&state.db)
    .await?;

    admin_auth::audit(
        &state,
        &actor,
        "admin_invitation_create",
        "admin_invitation",
        None,
        Some(invitation_id),
        &capabilities,
        "Admin team invitation was created.",
        json!({"email": email, "role": role, "expires_at": expires_at, "reason": reason}),
    )
    .await?;

    let mut invitation = fetch_invitation_for_actor(&state, invitation_id, &actor).await?;
    invitation.capabilities =
        admin_auth::sanitize_capabilities_for_role(&invitation.role, &invitation.capabilities);

    let return_token = std::env::var("SPARK_ADMIN_INVITE_RETURN_BOOTSTRAP_TOKENS")
        .ok()
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    Ok(success(CreateInvitationData {
        invitation,
        delivery_mode: if return_token {
            "manual_bootstrap"
        } else {
            "email_delivery_pending"
        },
        manual_token: if return_token { Some(token) } else { None },
    }))
}

async fn revoke_invitation(
    Path(invitation_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RevokeInvitationRequest>,
) -> Result<Json<AdminEnvelope<RevokeInvitationData>>, ApiError> {
    let actor = admin_auth::authorize_admin_actor(&state, &headers).await?;
    let invitation = fetch_invitation_for_actor(&state, invitation_id, &actor).await?;
    ensure_can_revoke_invitation(&actor, &invitation)?;
    if invitation.status != "pending" {
        return Err(ApiError::Conflict(
            "only pending invitations can be revoked".to_string(),
        ));
    }

    let reason = clean_reason(payload.reason.as_deref())?;
    sqlx::query(
        r#"
        update admin_invitations
        set revoked_at = now(),
            metadata = metadata || $2::jsonb
        where id = $1
          and accepted_at is null
          and revoked_at is null
          and expires_at > now()
        "#,
    )
    .bind(invitation_id)
    .bind(json!({"revocation_reason": reason, "revoked_by": actor.actor_kind}))
    .execute(&state.db)
    .await?;

    let revoked_capabilities = invitation.capabilities.clone();
    let revoked_email = invitation.email.clone();
    let revoked_role = invitation.role.clone();
    admin_auth::audit(
        &state,
        &actor,
        "admin_invitation_revoke",
        "admin_invitation",
        None,
        Some(invitation_id),
        &revoked_capabilities,
        "Admin team invitation was revoked.",
        json!({"email": revoked_email, "role": revoked_role, "reason": reason}),
    )
    .await?;

    let revoked = fetch_invitation_for_actor(&state, invitation_id, &actor).await?;
    Ok(success(RevokeInvitationData { invitation: revoked }))
}

async fn authorize_invitation_reader(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<admin_auth::AdminContext, ApiError> {
    let actor = admin_auth::authorize_admin_actor(state, headers).await?;
    if actor.role == "superadmin" || actor.role == "admin" {
        return Ok(actor);
    }
    Err(ApiError::Unauthorized)
}

fn ensure_can_invite_role(actor: &admin_auth::AdminContext, target_role: &str) -> Result<(), ApiError> {
    match (actor.role.as_str(), target_role) {
        ("superadmin", "admin" | "moderator") => Ok(()),
        ("admin", "moderator") => Ok(()),
        ("admin", "admin") => Err(ApiError::Unauthorized),
        ("moderator", _) => Err(ApiError::Unauthorized),
        _ => Err(ApiError::Unauthorized),
    }
}

fn ensure_can_revoke_invitation(
    actor: &admin_auth::AdminContext,
    invitation: &AdminInvitationRow,
) -> Result<(), ApiError> {
    if actor.role == "superadmin" {
        return Ok(());
    }
    if actor.role == "admin"
        && invitation.role == "moderator"
        && invitation.invited_by_user_id == actor.actor_user_id
    {
        return Ok(());
    }
    Err(ApiError::Unauthorized)
}

async fn fetch_invitation_for_actor(
    state: &AppState,
    invitation_id: Uuid,
    actor: &admin_auth::AdminContext,
) -> Result<AdminInvitationRow, ApiError> {
    let mut invitation = sqlx::query_as::<_, AdminInvitationRow>(
        r#"
        select id,
               email,
               role,
               capabilities,
               invited_by_actor_kind,
               invited_by_user_id,
               expires_at,
               accepted_at,
               accepted_by_user_id,
               revoked_at,
               created_at,
               metadata,
               case
                 when accepted_at is not null then 'accepted'
                 when revoked_at is not null then 'revoked'
                 when expires_at <= now() then 'expired'
                 else 'pending'
               end as status
        from admin_invitations
        where id = $1
          and (
            $2::text = 'superadmin'
            or (role = 'moderator' and invited_by_user_id = $3)
          )
        "#,
    )
    .bind(invitation_id)
    .bind(&actor.role)
    .bind(actor.actor_user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("invitation was not found".to_string()))?;

    invitation.capabilities =
        admin_auth::sanitize_capabilities_for_role(&invitation.role, &invitation.capabilities);
    Ok(invitation)
}

async fn fetch_active_assignment(
    state: &AppState,
    user_id: Uuid,
    role: &str,
) -> Result<admin_auth::AdminAssignmentRow, ApiError> {
    let mut assignment = sqlx::query_as::<_, admin_auth::AdminAssignmentRow>(
        r#"
        select ara.id,
               ara.user_id,
               u.email,
               coalesce(nullif(p.display_name, ''), split_part(u.email, '@', 1), 'Pengguna Spark') as display_name,
               p.handle,
               case when ara.role = 'sub_admin' then 'admin' else ara.role end as role,
               ara.capabilities,
               'active' as status,
               ara.reason,
               ara.starts_at,
               ara.expires_at,
               ara.created_at,
               ara.updated_at
        from admin_role_assignments ara
        join users u on u.id = ara.user_id
        left join profiles p on p.user_id = ara.user_id
        where ara.user_id = $1
          and (ara.role = $2 or ($2 = 'admin' and ara.role = 'sub_admin'))
          and ara.status = 'active'
          and ara.revoked_at is null
          and ara.starts_at <= now()
          and (ara.expires_at is null or ara.expires_at > now())
        order by case ara.role when 'admin' then 2 when 'sub_admin' then 2 when 'moderator' then 1 else 0 end desc,
                 ara.updated_at desc
        limit 1
        "#,
    )
    .bind(user_id)
    .bind(role)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("active admin assignment was not found".to_string()))?;

    assignment.capabilities =
        admin_auth::sanitize_capabilities_for_role(&assignment.role, &assignment.capabilities);
    Ok(assignment)
}

fn normalize_status(input: &str) -> Result<String, ApiError> {
    match input.trim() {
        "active" => Ok("active".to_string()),
        "revoked" => Ok("revoked".to_string()),
        "expired" => Ok("expired".to_string()),
        _ => Err(ApiError::BadRequest(
            "status must be active, revoked, or expired".to_string(),
        )),
    }
}

fn normalize_invitation_status(input: &str) -> Result<String, ApiError> {
    match input.trim() {
        "pending" => Ok("pending".to_string()),
        "accepted" => Ok("accepted".to_string()),
        "revoked" => Ok("revoked".to_string()),
        "expired" => Ok("expired".to_string()),
        "all" => Ok("all".to_string()),
        _ => Err(ApiError::BadRequest(
            "status must be pending, accepted, revoked, expired, or all".to_string(),
        )),
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
        Err(ApiError::BadRequest("valid email is required".to_string()))
    }
}

fn clean_reason(input: Option<&str>) -> Result<String, ApiError> {
    let value = input.unwrap_or("").trim();
    if value.chars().count() > 1000 {
        return Err(ApiError::BadRequest("reason is too long".to_string()));
    }
    if value.chars().any(char::is_control) {
        return Err(ApiError::BadRequest(
            "reason cannot contain control characters".to_string(),
        ));
    }
    Ok(value.to_string())
}

fn new_invite_token() -> String {
    format!("adm_inv_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn role_catalog() -> Vec<RoleInfo> {
    vec![
        RoleInfo {
            role: "superadmin",
            description: "Bootstrap/developer-level access controlled by the server admin token.",
            capabilities: admin_auth::SUPER_ADMIN_CAPABILITIES
                .iter()
                .map(|value| value.to_string())
                .collect(),
        },
        RoleInfo {
            role: "admin",
            description: "Operational admin role for CMS, moderation, bulk actions, and ML moderation workflows.",
            capabilities: admin_auth::ADMIN_ALLOWED_CAPABILITIES
                .iter()
                .map(|value| value.to_string())
                .collect(),
        },
        RoleInfo {
            role: "moderator",
            description:
                "Review-focused moderation role for reports, content inspection, and media review.",
            capabilities: admin_auth::MODERATOR_ALLOWED_CAPABILITIES
                .iter()
                .map(|value| value.to_string())
                .collect(),
        },
    ]
}
