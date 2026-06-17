use axum::http::HeaderMap;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{auth::session::require_current_user, error::ApiError, state::AppState};

pub const ADMIN_HEADER: &str = "x-karyra-admin-token";

pub const SUPER_ADMIN_CAPABILITIES: &[&str] = &[
    "developer_access",
    "admin_manage",
    "policy_manage",
    "ai_manage",
    "ml_moderation_manage",
    "moderation_read",
    "moderation_action",
    "moderation_restore",
    "moderation_bulk",
    "user_safety_manage",
    "reports_manage",
    "content_read",
    "content_create",
    "content_edit",
    "content_publish",
    "content_archive",
    "media_review",
    "audit_read",
];

pub const ADMIN_ALLOWED_CAPABILITIES: &[&str] = &[
    "policy_manage",
    "ai_manage",
    "ml_moderation_manage",
    "moderation_read",
    "moderation_action",
    "moderation_restore",
    "moderation_bulk",
    "user_safety_manage",
    "reports_manage",
    "content_read",
    "content_create",
    "content_edit",
    "content_publish",
    "content_archive",
    "media_review",
    "audit_read",
];

// Kept as a compatibility alias for older rows/API clients that used `sub_admin`.
pub const SUB_ADMIN_ALLOWED_CAPABILITIES: &[&str] = ADMIN_ALLOWED_CAPABILITIES;

pub const MODERATOR_ALLOWED_CAPABILITIES: &[&str] = &[
    "moderation_read",
    "moderation_action",
    "moderation_bulk",
    "reports_manage",
    "content_read",
    "media_review",
    "audit_read",
];

#[derive(Debug, Clone, Serialize)]
pub struct AdminContext {
    pub actor_kind: String,
    pub actor_user_id: Option<Uuid>,
    pub role: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AdminAssignmentRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub handle: Option<String>,
    pub role: String,
    pub capabilities: Vec<String>,
    pub status: String,
    pub reason: String,
    pub starts_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub fn all_capabilities() -> Vec<String> {
    SUPER_ADMIN_CAPABILITIES
        .iter()
        .map(|value| value.to_string())
        .collect()
}

pub fn canonical_role(role: &str) -> String {
    match role.trim() {
        "sub_admin" => "admin".to_string(),
        value => value.to_string(),
    }
}

pub fn allowed_capabilities_for_role(role: &str) -> Option<&'static [&'static str]> {
    match role.trim() {
        "admin" | "sub_admin" => Some(ADMIN_ALLOWED_CAPABILITIES),
        "moderator" => Some(MODERATOR_ALLOWED_CAPABILITIES),
        _ => None,
    }
}

pub fn normalize_role(input: &str) -> Result<String, ApiError> {
    match input.trim() {
        "admin" | "sub_admin" => Ok("admin".to_string()),
        "moderator" => Ok("moderator".to_string()),
        _ => Err(ApiError::BadRequest(
            "delegated admin role must be admin or moderator".to_string(),
        )),
    }
}

pub fn normalize_capabilities(role: &str, input: &[String]) -> Result<Vec<String>, ApiError> {
    let allowed = allowed_capabilities_for_role(role)
        .ok_or_else(|| ApiError::BadRequest("admin role is not assignable".to_string()))?;
    let mut output = Vec::<String>::new();

    for item in input {
        let capability = item.trim();
        if capability.is_empty() {
            continue;
        }
        if !allowed
            .iter()
            .any(|allowed_item| allowed_item == &capability)
        {
            return Err(ApiError::BadRequest(format!(
                "capability {capability} is not allowed for role {role}"
            )));
        }
        if !output.iter().any(|value| value == capability) {
            output.push(capability.to_string());
        }
    }

    if output.is_empty() {
        output = default_capabilities_for_role(role)?;
    }

    Ok(output)
}

pub fn default_capabilities_for_role(role: &str) -> Result<Vec<String>, ApiError> {
    let defaults: &[&str] = match role.trim() {
        "admin" | "sub_admin" => &[
            "moderation_read",
            "moderation_action",
            "moderation_restore",
            "moderation_bulk",
            "reports_manage",
            "content_read",
            "content_create",
            "content_edit",
            "content_publish",
            "content_archive",
            "media_review",
            "audit_read",
        ],
        "moderator" => &[
            "moderation_read",
            "moderation_action",
            "moderation_bulk",
            "reports_manage",
            "content_read",
            "media_review",
        ],
        _ => {
            return Err(ApiError::BadRequest(
                "admin role is not assignable".to_string(),
            ))
        }
    };
    Ok(defaults.iter().map(|value| value.to_string()).collect())
}

pub async fn authorize_with_capability(
    state: &AppState,
    headers: &HeaderMap,
    capability: &str,
) -> Result<AdminContext, ApiError> {
    if let Some(context) = authorize_super_admin_token(state, headers)? {
        return Ok(context);
    }

    let user = require_current_user(state, headers).await?;
    let row = sqlx::query_as::<_, AssignmentForAuth>(
        r#"
        select role, capabilities
        from admin_role_assignments
        where user_id = $1
          and role in ('admin', 'sub_admin', 'moderator')
          and status = 'active'
          and revoked_at is null
          and starts_at <= now()
          and (expires_at is null or expires_at > now())
        order by case role when 'admin' then 2 when 'sub_admin' then 2 when 'moderator' then 1 else 0 end desc,
                 updated_at desc
        limit 1
        "#,
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::Unauthorized)?;

    if !row.capabilities.iter().any(|item| item == capability) {
        return Err(ApiError::Unauthorized);
    }

    let role = canonical_role(&row.role);
    Ok(AdminContext {
        actor_kind: role.clone(),
        actor_user_id: Some(user.id),
        role,
        capabilities: row.capabilities,
    })
}

pub async fn authorize_admin_manage(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AdminContext, ApiError> {
    authorize_with_capability(state, headers, "admin_manage").await
}

pub fn authorize_super_admin_only(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AdminContext, ApiError> {
    authorize_super_admin_token(state, headers)?.ok_or(ApiError::Unauthorized)
}

fn authorize_super_admin_token(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Option<AdminContext>, ApiError> {
    let Some(configured) = state.config.admin_token.as_deref() else {
        return Ok(None);
    };
    let Some(supplied) = headers
        .get(ADMIN_HEADER)
        .and_then(|value| value.to_str().ok())
    else {
        return Ok(None);
    };

    if Sha256::digest(configured.as_bytes()) == Sha256::digest(supplied.as_bytes()) {
        Ok(Some(AdminContext {
            actor_kind: "superadmin_token".to_string(),
            actor_user_id: None,
            role: "superadmin".to_string(),
            capabilities: all_capabilities(),
        }))
    } else {
        Err(ApiError::Unauthorized)
    }
}

#[derive(Debug, FromRow)]
struct AssignmentForAuth {
    role: String,
    capabilities: Vec<String>,
}

pub async fn audit(
    state: &AppState,
    actor: &AdminContext,
    action: &str,
    target_type: &str,
    target_user_id: Option<Uuid>,
    target_id: Option<Uuid>,
    capabilities: &[String],
    summary: &str,
    metadata: serde_json::Value,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        insert into admin_audit_events (
          id, actor_kind, actor_user_id, action, target_type, target_user_id,
          target_id, capabilities, summary, metadata
        ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&actor.actor_kind)
    .bind(actor.actor_user_id)
    .bind(action)
    .bind(target_type)
    .bind(target_user_id)
    .bind(target_id)
    .bind(capabilities.to_vec())
    .bind(summary)
    .bind(metadata)
    .execute(&state.db)
    .await?;
    Ok(())
}
