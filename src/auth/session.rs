use axum::http::{header, HeaderMap};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: Uuid,
}

#[derive(Debug, FromRow)]
struct CurrentUserRow {
    id: Uuid,
}

pub async fn require_current_user(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<CurrentUser, ApiError> {
    let token = read_session_cookie(headers, &state.config.session_cookie_name)?;
    let token_hash = hash_session_token(&token);

    let row = sqlx::query_as::<_, CurrentUserRow>(
        r#"
        update sessions
        set last_seen_at = now()
        from users
        where sessions.user_id = users.id
          and sessions.token_hash = $1
          and sessions.expires_at > now()
          and sessions.revoked_at is null
          and users.status = 'active'
        returning users.id
        "#,
    )
    .bind(token_hash)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::Unauthorized)?;

    Ok(CurrentUser { id: row.id })
}

pub fn read_session_cookie(headers: &HeaderMap, cookie_name: &str) -> Result<String, ApiError> {
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

pub fn hash_session_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}
