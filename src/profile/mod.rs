use axum::{extract::State, http::HeaderMap, routing::get, Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{auth::session::require_current_user, error::ApiError, state::AppState};

const DEFAULT_VISIBILITY: &str = "community";
const DEFAULT_AVATAR_PRESET: &str = "spark";

#[derive(Serialize)]
struct ProfileScopeResponse {
    module: &'static str,
    phase: &'static str,
    implemented_now: Vec<&'static str>,
    privacy_note: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub display_name: Option<String>,
    pub handle: Option<String>,
    pub bio: Option<String>,
    pub location: Option<String>,
    pub visibility: Option<String>,
    pub avatar_preset: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct ProfileResponse {
    user_id: Uuid,
    email: String,
    display_name: String,
    handle: Option<String>,
    bio: String,
    location: String,
    visibility: String,
    avatar_preset: String,
    avatar_url: Option<String>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct ProfileRow {
    user_id: Uuid,
    email: String,
    display_name: String,
    handle: Option<String>,
    bio: String,
    location: String,
    visibility: String,
    avatar_preset: String,
    avatar_url: Option<String>,
    updated_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/scope", get(scope))
        .route("/me", get(me).post(update_me))
}

async fn scope() -> Json<ProfileScopeResponse> {
    Json(ProfileScopeResponse {
        module: module_path!(),
        phase: "profile-account-runtime",
        implemented_now: vec![
            "authenticated-profile-read",
            "authenticated-profile-update",
            "profile-is-separate-from-passport",
            "frontend-profile-backend-hydration",
        ],
        privacy_note: "Profile stores account identity preferences only. Passport remains a readiness credential.",
    })
}

async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ProfileResponse>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    ensure_profile(&state, user.id).await?;
    let row = fetch_profile(&state, user.id).await?;
    Ok(Json(ProfileResponse::from(row)))
}

async fn update_me(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateProfileRequest>,
) -> Result<Json<ProfileResponse>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let existing = ensure_profile(&state, user.id).await?;

    let display_name = match payload.display_name.as_deref() {
        Some(value) => clean_text(value, "display_name", 2, 64)?,
        None => existing.display_name,
    };
    let handle = match payload.handle.as_deref() {
        Some(value) => clean_optional_handle(value)?,
        None => existing.handle,
    };
    let bio = match payload.bio.as_deref() {
        Some(value) => clean_optional_text(value, "bio", 240)?,
        None => existing.bio,
    };
    let location = match payload.location.as_deref() {
        Some(value) => clean_optional_text(value, "location", 96)?,
        None => existing.location,
    };
    let visibility = match payload.visibility.as_deref() {
        Some(value) => normalize_visibility(value)?,
        None => existing.visibility,
    };
    let avatar_preset = match payload.avatar_preset.as_deref() {
        Some(value) => normalize_avatar_preset(value)?,
        None => existing.avatar_preset,
    };
    let avatar_url = match payload.avatar_url.as_deref() {
        Some(value) => clean_optional_url(value)?,
        None => existing.avatar_url,
    };

    let row = sqlx::query_as::<_, ProfileRow>(
        r#"
        insert into profiles (
            user_id,
            display_name,
            handle,
            bio,
            location,
            visibility,
            avatar_preset,
            avatar_url
        )
        values ($1, $2, $3, $4, $5, $6, $7, $8)
        on conflict (user_id) do update set
          display_name = excluded.display_name,
          handle = excluded.handle,
          bio = excluded.bio,
          location = excluded.location,
          visibility = excluded.visibility,
          avatar_preset = excluded.avatar_preset,
          avatar_url = excluded.avatar_url,
          updated_at = now()
        returning user_id,
                  (select email from users where id = profiles.user_id) as email,
                  display_name,
                  handle,
                  coalesce(bio, '') as bio,
                  coalesce(location, '') as location,
                  coalesce(visibility, $9) as visibility,
                  coalesce(avatar_preset, $10) as avatar_preset,
                  avatar_url,
                  updated_at
        "#,
    )
    .bind(user.id)
    .bind(display_name)
    .bind(handle)
    .bind(bio)
    .bind(location)
    .bind(visibility)
    .bind(avatar_preset)
    .bind(avatar_url)
    .bind(DEFAULT_VISIBILITY)
    .bind(DEFAULT_AVATAR_PRESET)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(ProfileResponse::from(row)))
}

async fn ensure_profile(state: &AppState, user_id: Uuid) -> Result<ProfileRow, ApiError> {
    sqlx::query(
        r#"
        insert into profiles (user_id, display_name, visibility, avatar_preset)
        select id,
               coalesce(nullif(split_part(email, '@', 1), ''), 'Pengguna Spark'),
               $2,
               $3
        from users
        where id = $1
        on conflict (user_id) do nothing
        "#,
    )
    .bind(user_id)
    .bind(DEFAULT_VISIBILITY)
    .bind(DEFAULT_AVATAR_PRESET)
    .execute(&state.db)
    .await?;

    fetch_profile(state, user_id).await
}

async fn fetch_profile(state: &AppState, user_id: Uuid) -> Result<ProfileRow, ApiError> {
    let row = sqlx::query_as::<_, ProfileRow>(
        r#"
        select users.id as user_id,
               users.email,
               coalesce(nullif(profiles.display_name, ''), split_part(users.email, '@', 1), 'Pengguna Spark') as display_name,
               profiles.handle,
               coalesce(profiles.bio, '') as bio,
               coalesce(profiles.location, '') as location,
               coalesce(profiles.visibility, $2) as visibility,
               coalesce(profiles.avatar_preset, $3) as avatar_preset,
               profiles.avatar_url,
               coalesce(profiles.updated_at, users.created_at) as updated_at
        from users
        left join profiles on profiles.user_id = users.id
        where users.id = $1 and users.status = 'active'
        "#,
    )
    .bind(user_id)
    .bind(DEFAULT_VISIBILITY)
    .bind(DEFAULT_AVATAR_PRESET)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::Unauthorized)?;

    Ok(row)
}

fn clean_text(input: &str, field: &str, min: usize, max: usize) -> Result<String, ApiError> {
    let value = input.trim();
    let len = value.chars().count();
    if len < min {
        return Err(ApiError::BadRequest(format!("{field} is too short")));
    }
    if len > max {
        return Err(ApiError::BadRequest(format!("{field} is too long")));
    }
    if value.chars().any(char::is_control) {
        return Err(ApiError::BadRequest(format!(
            "{field} cannot contain control characters"
        )));
    }
    Ok(value.to_string())
}

fn clean_optional_text(input: &str, field: &str, max: usize) -> Result<String, ApiError> {
    let value = input.trim();
    if value.is_empty() {
        return Ok(String::new());
    }
    clean_text(value, field, 0, max)
}

fn clean_optional_handle(input: &str) -> Result<Option<String>, ApiError> {
    let raw = input.trim().trim_start_matches('@').to_ascii_lowercase();
    if raw.is_empty() {
        return Ok(None);
    }
    if raw.chars().count() > 32 {
        return Err(ApiError::BadRequest("handle is too long".to_string()));
    }
    if raw.chars().any(|character| {
        !(character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
    }) {
        return Err(ApiError::BadRequest(
            "handle can only contain letters, numbers, dot, dash, and underscore".to_string(),
        ));
    }
    Ok(Some(format!("@{raw}")))
}

fn normalize_visibility(input: &str) -> Result<String, ApiError> {
    let value = input.trim().to_ascii_lowercase();
    match value.as_str() {
        "private" | "community" | "public" => Ok(value),
        _ => Err(ApiError::BadRequest(
            "visibility must be private, community, or public".to_string(),
        )),
    }
}

fn normalize_avatar_preset(input: &str) -> Result<String, ApiError> {
    let value = input.trim().to_ascii_lowercase();
    match value.as_str() {
        "spark" | "trophy" | "coffee" | "explorer" | "mentor" => Ok(value),
        _ => Err(ApiError::BadRequest(
            "avatar_preset is not supported".to_string(),
        )),
    }
}

fn clean_optional_url(input: &str) -> Result<Option<String>, ApiError> {
    let value = input.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > 512 {
        return Err(ApiError::BadRequest("avatar_url is too long".to_string()));
    }
    if value.chars().any(char::is_control) {
        return Err(ApiError::BadRequest(
            "avatar_url cannot contain control characters".to_string(),
        ));
    }
    Ok(Some(value.to_string()))
}

impl From<ProfileRow> for ProfileResponse {
    fn from(row: ProfileRow) -> Self {
        Self {
            user_id: row.user_id,
            email: row.email,
            display_name: row.display_name,
            handle: row.handle,
            bio: row.bio,
            location: row.location,
            visibility: row.visibility,
            avatar_preset: row.avatar_preset,
            avatar_url: row.avatar_url,
            updated_at: row.updated_at,
        }
    }
}
