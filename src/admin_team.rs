use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{admin_auth, error::ApiError, state::AppState};

#[derive(Serialize)]
struct AdminEnvelope<T> {
    ok: bool,
    data: T,
    generated_at: DateTime<Utc>,
}

fn success<T>(data: T) -> Json<AdminEnvelope<T>> {
    Json(AdminEnvelope { ok: true, data, generated_at: Utc::now() })
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/scope", get(scope))
        .route("/members", get(members).post(upsert_member))
        .route("/members/:user_id/revoke", post(revoke_member))
        .route("/capabilities", get(capabilities))
}

#[derive(Serialize)]
struct ScopeData {
    module: &'static str,
    phase: &'static str,
    roles: Vec<RoleInfo>,
    routes: Vec<&'static str>,
    auth_model: &'static str,
}

#[derive(Serialize)]
struct RoleInfo {
    role: &'static str,
    description: &'static str,
    capabilities: Vec<String>,
}

async fn scope(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<AdminEnvelope<ScopeData>>, ApiError> {
    admin_auth::authorize_with_capability(&state, &headers, "audit_read").await?;
    Ok(success(ScopeData {
        module: module_path!(),
        phase: "admin-rbac-foundation",
        roles: role_catalog(),
        routes: vec![
            "GET /api/admin/team/scope",
            "GET /api/admin/team/capabilities",
            "GET /api/admin/team/members",
            "POST /api/admin/team/members",
            "POST /api/admin/team/members/:user_id/revoke",
        ],
        auth_model: "super-admin token bootstrap plus session-based sub-admin/moderator assignments",
    }))
}

async fn capabilities(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<AdminEnvelope<Vec<RoleInfo>>>, ApiError> {
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
    let role = query.role.as_deref().map(admin_auth::normalize_role).transpose()?;
    let status = query.status.as_deref().map(normalize_status).transpose()?.unwrap_or_else(|| "active".to_string());

    let items = sqlx::query_as::<_, admin_auth::AdminAssignmentRow>(
        r#"
        select ara.id,
               ara.user_id,
               u.email,
               coalesce(nullif(p.display_name, ''), split_part(u.email, '@', 1), 'Pengguna Spark') as display_name,
               p.handle,
               ara.role,
               ara.capabilities,
               ara.status,
               ara.reason,
               ara.starts_at,
               ara.expires_at,
               ara.created_at,
               ara.updated_at
        from admin_role_assignments ara
        join users u on u.id = ara.user_id
        left join profiles p on p.user_id = ara.user_id
        where ($1::text is null or ara.role = $1)
          and ara.status = $2
          and ($2 <> 'active' or ara.revoked_at is null)
        order by ara.updated_at desc, ara.created_at desc
        limit $3 offset $4
        "#,
    )
    .bind(role.as_deref())
    .bind(&status)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total = sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)
        from admin_role_assignments ara
        where ($1::text is null or ara.role = $1)
          and ara.status = $2
          and ($2 <> 'active' or ara.revoked_at is null)
        "#,
    )
    .bind(role.as_deref())
    .bind(&status)
    .fetch_one(&state.db)
    .await?;

    Ok(success(MembersData { items, total, limit, offset }))
}

#[derive(Debug, Deserialize)]
struct UpsertMemberRequest {
    user_id: Option<Uuid>,
    email: Option<String>,
    role: String,
    capabilities: Option<Vec<String>>,
    reason: Option<String>,
    expires_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
struct MemberWriteData {
    assignment: admin_auth::AdminAssignmentRow,
}

async fn upsert_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpsertMemberRequest>,
) -> Result<Json<AdminEnvelope<MemberWriteData>>, ApiError> {
    let actor = admin_auth::authorize_admin_manage(&state, &headers).await?;
    let role = admin_auth::normalize_role(&payload.role)?;
    let requested_capabilities = payload.capabilities.unwrap_or_default();
    let capabilities = admin_auth::normalize_capabilities(&role, &requested_capabilities)?;
    let user_id = resolve_target_user(&state, payload.user_id, payload.email.as_deref()).await?;
    let reason = clean_reason(payload.reason.as_deref())?;

    let assignment_id = Uuid::new_v4();
    sqlx::query(
        r#"
        insert into admin_role_assignments (
          id, user_id, role, capabilities, status, granted_by_user_id,
          granted_by_kind, reason, starts_at, expires_at, revoked_at, metadata
        ) values ($1, $2, $3, $4, 'active', $5, $6, $7, now(), $8, null, $9)
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
    .bind(user_id)
    .bind(&role)
    .bind(capabilities.clone())
    .bind(actor.actor_user_id)
    .bind(&actor.actor_kind)
    .bind(&reason)
    .bind(payload.expires_at)
    .bind(json!({"updated_by": actor.actor_kind, "source": "admin_team_api"}))
    .execute(&state.db)
    .await?;

    admin_auth::audit(
        &state,
        &actor,
        "admin_role_upsert",
        "user",
        Some(user_id),
        None,
        &capabilities,
        "Admin role assignment was created or updated.",
        json!({"role": role, "reason": reason}),
    )
    .await?;

    let assignment = fetch_active_assignment(&state, user_id, &role).await?;
    Ok(success(MemberWriteData { assignment }))
}

#[derive(Debug, Deserialize)]
struct RevokeMemberRequest {
    role: String,
    reason: Option<String>,
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
        where user_id = $1 and role = $2 and status = 'active' and revoked_at is null
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

async fn resolve_target_user(state: &AppState, user_id: Option<Uuid>, email: Option<&str>) -> Result<Uuid, ApiError> {
    if let Some(id) = user_id {
        let exists = sqlx::query_scalar::<_, bool>("select exists(select 1 from users where id = $1 and status = 'active')")
            .bind(id)
            .fetch_one(&state.db)
            .await?;
        if exists {
            return Ok(id);
        }
        return Err(ApiError::BadRequest("target user was not found or is not active".to_string()));
    }

    let Some(email) = email else {
        return Err(ApiError::BadRequest("user_id or email is required".to_string()));
    };
    let email = email.trim().to_ascii_lowercase();
    if email.is_empty() || email.chars().count() > 320 || !email.contains('@') {
        return Err(ApiError::BadRequest("valid email is required".to_string()));
    }

    sqlx::query_scalar::<_, Uuid>("select id from users where lower(email) = lower($1) and status = 'active'")
        .bind(email)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::BadRequest("target user was not found or is not active".to_string()))
}

async fn fetch_active_assignment(state: &AppState, user_id: Uuid, role: &str) -> Result<admin_auth::AdminAssignmentRow, ApiError> {
    sqlx::query_as::<_, admin_auth::AdminAssignmentRow>(
        r#"
        select ara.id,
               ara.user_id,
               u.email,
               coalesce(nullif(p.display_name, ''), split_part(u.email, '@', 1), 'Pengguna Spark') as display_name,
               p.handle,
               ara.role,
               ara.capabilities,
               ara.status,
               ara.reason,
               ara.starts_at,
               ara.expires_at,
               ara.created_at,
               ara.updated_at
        from admin_role_assignments ara
        join users u on u.id = ara.user_id
        left join profiles p on p.user_id = ara.user_id
        where ara.user_id = $1 and ara.role = $2 and ara.status = 'active' and ara.revoked_at is null
        "#,
    )
    .bind(user_id)
    .bind(role)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("active admin assignment was not found".to_string()))
}

fn normalize_status(input: &str) -> Result<String, ApiError> {
    match input.trim() {
        "active" => Ok("active".to_string()),
        "revoked" => Ok("revoked".to_string()),
        "expired" => Ok("expired".to_string()),
        _ => Err(ApiError::BadRequest("status must be active, revoked, or expired".to_string())),
    }
}

fn clean_reason(input: Option<&str>) -> Result<String, ApiError> {
    let value = input.unwrap_or("").trim();
    if value.chars().count() > 1000 {
        return Err(ApiError::BadRequest("reason is too long".to_string()));
    }
    if value.chars().any(char::is_control) {
        return Err(ApiError::BadRequest("reason cannot contain control characters".to_string()));
    }
    Ok(value.to_string())
}

fn role_catalog() -> Vec<RoleInfo> {
    vec![
        RoleInfo {
            role: "super_admin",
            description: "Bootstrap/developer-level admin controlled by the existing server token.",
            capabilities: admin_auth::SUPER_ADMIN_CAPABILITIES.iter().map(|value| value.to_string()).collect(),
        },
        RoleInfo {
            role: "sub_admin",
            description: "Delegated admin role with configurable capabilities below super-admin level.",
            capabilities: admin_auth::SUB_ADMIN_ALLOWED_CAPABILITIES.iter().map(|value| value.to_string()).collect(),
        },
        RoleInfo {
            role: "moderator",
            description: "Moderation-focused role with configurable review/action capabilities.",
            capabilities: admin_auth::MODERATOR_ALLOWED_CAPABILITIES.iter().map(|value| value.to_string()).collect(),
        },
    ]
}
