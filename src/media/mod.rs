use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{auth::session::require_current_user, error::ApiError, state::AppState};

const MAX_UPLOAD_BYTES: i64 = 8 * 1024 * 1024;
const UPLOAD_TTL_MINUTES: i64 = 15;
const STORAGE_PROVIDER: &str = "s3-compatible";
const UPLOAD_METHOD: &str = "PUT";

#[derive(Serialize)]
struct MediaPolicyResponse {
    provider: &'static str,
    current_phase: &'static str,
    storage_default: &'static str,
    production_small: &'static str,
    future_scale: &'static str,
    max_upload_bytes: i64,
    upload_ttl_minutes: i64,
    allowed_mime_prefixes: Vec<&'static str>,
    accepted_purposes: Vec<&'static str>,
    physical_buckets: Vec<&'static str>,
    note: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct UploadIntentRequest {
    pub purpose: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub private: Option<bool>,
    pub checksum: Option<String>,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct CompleteUploadRequest {
    pub checksum: Option<String>,
    pub size_bytes: Option<i64>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMediaLinkRequest {
    pub entity_type: String,
    pub entity_id: String,
    pub purpose: String,
}

#[derive(Debug, Serialize)]
struct UploadIntentResponse {
    asset: MediaAssetResponse,
    provider: &'static str,
    upload_method: &'static str,
    upload_url: String,
    expires_at: DateTime<Utc>,
    note: &'static str,
}

#[derive(Debug, Serialize)]
struct MediaAssetResponse {
    id: Uuid,
    bucket: String,
    object_key: String,
    original_file_name: String,
    mime_type: String,
    size_bytes: i64,
    checksum: Option<String>,
    visibility: String,
    status: String,
    storage_provider: String,
    upload_method: String,
    upload_expires_at: Option<DateTime<Utc>>,
    uploaded_at: Option<DateTime<Utc>>,
    public_url: Option<String>,
    metadata: Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct MediaAssetRow {
    id: Uuid,
    bucket: String,
    object_key: String,
    original_file_name: String,
    mime_type: String,
    size_bytes: i64,
    checksum: Option<String>,
    visibility: String,
    status: String,
    storage_provider: String,
    upload_method: String,
    upload_expires_at: Option<DateTime<Utc>>,
    uploaded_at: Option<DateTime<Utc>>,
    public_url: Option<String>,
    metadata: Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct MediaLinkResponse {
    id: Uuid,
    media_asset_id: Uuid,
    entity_type: String,
    entity_id: String,
    purpose: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct MediaLinkRow {
    id: Uuid,
    media_asset_id: Uuid,
    entity_type: String,
    entity_id: String,
    purpose: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ListResponse<T> {
    items: Vec<T>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/policy", get(policy))
        .route("/me/assets", get(list_my_assets))
        .route("/upload-intents", post(create_upload_intent))
        .route("/assets/:asset_id", get(get_my_asset))
        .route("/assets/:asset_id/complete", post(complete_upload))
        .route("/assets/:asset_id/links", post(create_media_link))
}

async fn policy() -> Json<MediaPolicyResponse> {
    Json(MediaPolicyResponse {
        provider: STORAGE_PROVIDER,
        current_phase: "media-upload-foundation",
        storage_default: "MinIO for local development and small production",
        production_small: "Garage or MinIO self-hosted",
        future_scale: "Cloudflare R2/S3-compatible provider when needed",
        max_upload_bytes: MAX_UPLOAD_BYTES,
        upload_ttl_minutes: UPLOAD_TTL_MINUTES,
        allowed_mime_prefixes: vec!["image/", "application/pdf", "text/"],
        accepted_purposes: accepted_purposes(),
        physical_buckets: vec!["spark-public", "spark-private"],
        note: "This pass persists upload intents and media links. Real signed S3 URL generation remains a deploy/server integration step.",
    })
}

async fn list_my_assets(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ListResponse<MediaAssetResponse>>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let rows = sqlx::query_as::<_, MediaAssetRow>(
        r#"
        select id,
               bucket,
               object_key,
               original_file_name,
               mime_type,
               size_bytes,
               checksum,
               visibility,
               status,
               storage_provider,
               upload_method,
               upload_expires_at,
               uploaded_at,
               public_url,
               metadata,
               created_at,
               updated_at
        from media_assets
        where owner_user_id = $1
        order by created_at desc
        limit 100
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(ListResponse {
        items: rows.into_iter().map(MediaAssetResponse::from).collect(),
    }))
}

async fn get_my_asset(
    Path(asset_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MediaAssetResponse>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let row = fetch_owned_asset(&state, user.id, asset_id).await?;
    Ok(Json(MediaAssetResponse::from(row)))
}

async fn create_upload_intent(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UploadIntentRequest>,
) -> Result<(StatusCode, Json<UploadIntentResponse>), ApiError> {
    let user = require_current_user(&state, &headers).await?;
    validate_upload_request(&payload)?;

    let asset_id = Uuid::new_v4();
    let purpose = normalize_purpose(&payload.purpose)?;
    let file_name = sanitize_object_segment(&payload.file_name, "upload.bin");
    let mime_type = normalize_mime_type(&payload.mime_type)?;
    let checksum = clean_optional_segment(payload.checksum.as_deref(), "checksum")?;
    let entity_type = clean_optional_segment(payload.entity_type.as_deref(), "entity_type")?;
    let entity_id = clean_optional_segment(payload.entity_id.as_deref(), "entity_id")?;
    let private = payload
        .private
        .unwrap_or_else(|| default_private_for_purpose(&purpose));
    let visibility = if private { "private" } else { "public" };
    let bucket = if private {
        state.config.s3_bucket_private.clone()
    } else {
        state.config.s3_bucket_public.clone()
    };
    let object_key = format!("{purpose}/{}/{asset_id}/{file_name}", user.id);
    let upload_url = storage_url(&state.config.s3_endpoint, &bucket, &object_key);
    let upload_expires_at = Utc::now() + Duration::minutes(UPLOAD_TTL_MINUTES);
    let public_url = if visibility == "public" {
        Some(upload_url.clone())
    } else {
        None
    };
    let metadata = json!({
        "purpose": purpose,
        "entity_type": entity_type,
        "entity_id": entity_id,
        "client_metadata": payload.metadata.unwrap_or_else(|| json!({})),
        "source": "media.upload_intent"
    });

    let row = sqlx::query_as::<_, MediaAssetRow>(
        r#"
        insert into media_assets (
            id,
            owner_user_id,
            bucket,
            object_key,
            original_file_name,
            mime_type,
            size_bytes,
            checksum,
            visibility,
            status,
            storage_provider,
            upload_method,
            upload_expires_at,
            public_url,
            metadata
        )
        values ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'pending', $10, $11, $12, $13, $14)
        returning id,
                  bucket,
                  object_key,
                  original_file_name,
                  mime_type,
                  size_bytes,
                  checksum,
                  visibility,
                  status,
                  storage_provider,
                  upload_method,
                  upload_expires_at,
                  uploaded_at,
                  public_url,
                  metadata,
                  created_at,
                  updated_at
        "#,
    )
    .bind(asset_id)
    .bind(user.id)
    .bind(bucket)
    .bind(object_key)
    .bind(file_name)
    .bind(mime_type)
    .bind(payload.size_bytes)
    .bind(checksum)
    .bind(visibility)
    .bind(STORAGE_PROVIDER)
    .bind(UPLOAD_METHOD)
    .bind(upload_expires_at.clone())
    .bind(public_url)
    .bind(metadata)
    .fetch_one(&state.db)
    .await?;

    let response = UploadIntentResponse {
        provider: STORAGE_PROVIDER,
        upload_method: UPLOAD_METHOD,
        upload_url,
        expires_at: upload_expires_at,
        asset: MediaAssetResponse::from(row),
        note: "Upload intent stored. Client can PUT to the returned S3-compatible URL in dev/self-hosted setups; signed URL hardening happens on server deployment.",
    };

    Ok((StatusCode::CREATED, Json(response)))
}

async fn complete_upload(
    Path(asset_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CompleteUploadRequest>,
) -> Result<Json<MediaAssetResponse>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let checksum = clean_optional_segment(payload.checksum.as_deref(), "checksum")?;
    let size_bytes = payload
        .size_bytes
        .map(|value| validate_size_bytes(value, "size_bytes"))
        .transpose()?;
    let metadata = payload.metadata;

    let row = sqlx::query_as::<_, MediaAssetRow>(
        r#"
        update media_assets
        set status = 'uploaded',
            uploaded_at = coalesce(uploaded_at, now()),
            checksum = coalesce($3, checksum),
            size_bytes = coalesce($4, size_bytes),
            metadata = coalesce($5, metadata),
            updated_at = now()
        where id = $1 and owner_user_id = $2
        returning id,
                  bucket,
                  object_key,
                  original_file_name,
                  mime_type,
                  size_bytes,
                  checksum,
                  visibility,
                  status,
                  storage_provider,
                  upload_method,
                  upload_expires_at,
                  uploaded_at,
                  public_url,
                  metadata,
                  created_at,
                  updated_at
        "#,
    )
    .bind(asset_id)
    .bind(user.id)
    .bind(checksum)
    .bind(size_bytes)
    .bind(metadata)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("media asset not found".to_string()))?;

    Ok(Json(MediaAssetResponse::from(row)))
}

async fn create_media_link(
    Path(asset_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateMediaLinkRequest>,
) -> Result<(StatusCode, Json<MediaLinkResponse>), ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let _asset = fetch_owned_asset(&state, user.id, asset_id).await?;
    let entity_type = clean_text_segment(&payload.entity_type, "entity_type")?;
    let entity_id = clean_text_segment(&payload.entity_id, "entity_id")?;
    let purpose = normalize_purpose(&payload.purpose)?;

    let row = sqlx::query_as::<_, MediaLinkRow>(
        r#"
        insert into media_links (media_asset_id, entity_type, entity_id, purpose)
        values ($1, $2, $3, $4)
        returning id, media_asset_id, entity_type, entity_id, purpose, created_at
        "#,
    )
    .bind(asset_id)
    .bind(entity_type)
    .bind(entity_id)
    .bind(purpose)
    .fetch_one(&state.db)
    .await?;

    Ok((StatusCode::CREATED, Json(MediaLinkResponse::from(row))))
}

async fn fetch_owned_asset(
    state: &AppState,
    user_id: Uuid,
    asset_id: Uuid,
) -> Result<MediaAssetRow, ApiError> {
    let row = sqlx::query_as::<_, MediaAssetRow>(
        r#"
        select id,
               bucket,
               object_key,
               original_file_name,
               mime_type,
               size_bytes,
               checksum,
               visibility,
               status,
               storage_provider,
               upload_method,
               upload_expires_at,
               uploaded_at,
               public_url,
               metadata,
               created_at,
               updated_at
        from media_assets
        where id = $1 and owner_user_id = $2
        "#,
    )
    .bind(asset_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("media asset not found".to_string()))?;

    Ok(row)
}

fn validate_upload_request(payload: &UploadIntentRequest) -> Result<(), ApiError> {
    normalize_purpose(&payload.purpose)?;
    clean_text_segment(&payload.file_name, "file_name")?;
    normalize_mime_type(&payload.mime_type)?;
    validate_size_bytes(payload.size_bytes, "size_bytes")?;
    clean_optional_segment(payload.checksum.as_deref(), "checksum")?;
    clean_optional_segment(payload.entity_type.as_deref(), "entity_type")?;
    clean_optional_segment(payload.entity_id.as_deref(), "entity_id")?;
    Ok(())
}

fn validate_size_bytes(value: i64, field: &str) -> Result<i64, ApiError> {
    if value <= 0 {
        return Err(ApiError::BadRequest(format!(
            "{field} must be greater than zero"
        )));
    }

    if value > MAX_UPLOAD_BYTES {
        return Err(ApiError::BadRequest(format!(
            "{field} exceeds MVP limit of {MAX_UPLOAD_BYTES} bytes"
        )));
    }

    Ok(value)
}

fn normalize_mime_type(input: &str) -> Result<String, ApiError> {
    let value = input.trim().to_ascii_lowercase();
    let allowed =
        value.starts_with("image/") || value == "application/pdf" || value.starts_with("text/");

    if allowed && value.chars().count() <= 128 && !value.chars().any(char::is_control) {
        Ok(value)
    } else {
        Err(ApiError::BadRequest(
            "mime_type must be image/*, application/pdf, or text/*".to_string(),
        ))
    }
}

fn normalize_purpose(input: &str) -> Result<String, ApiError> {
    let value = sanitize_object_segment(input, "general").to_ascii_lowercase();

    if accepted_purposes().contains(&value.as_str()) {
        Ok(value)
    } else {
        Err(ApiError::BadRequest(format!(
            "purpose must be one of: {}",
            accepted_purposes().join(", ")
        )))
    }
}

fn accepted_purposes() -> Vec<&'static str> {
    vec![
        "avatar",
        "profile",
        "lesson",
        "lab",
        "community",
        "event",
        "passport",
        "evidence",
        "proof",
        "general",
    ]
}

fn default_private_for_purpose(purpose: &str) -> bool {
    matches!(purpose, "evidence" | "proof" | "lab")
}

fn clean_optional_segment(input: Option<&str>, field: &str) -> Result<Option<String>, ApiError> {
    input
        .map(|value| clean_text_segment(value, field))
        .transpose()
}

fn clean_text_segment(input: &str, field: &str) -> Result<String, ApiError> {
    let value = input.trim();

    if value.is_empty() {
        return Err(ApiError::BadRequest(format!("{field} is required")));
    }

    if value.chars().count() > 128 {
        return Err(ApiError::BadRequest(format!("{field} is too long")));
    }

    if value.chars().any(char::is_control) {
        return Err(ApiError::BadRequest(format!(
            "{field} cannot contain control characters"
        )));
    }

    Ok(value.to_string())
}

fn sanitize_object_segment(input: &str, fallback: &str) -> String {
    let mut output = String::with_capacity(input.len());

    for character in input.trim().chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
            output.push(character);
        } else {
            output.push('-');
        }
    }

    let trimmed = output.trim_matches(&['-', '.'][..]).to_string();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.chars().take(96).collect()
    }
}

fn storage_url(endpoint: &str, bucket: &str, object_key: &str) -> String {
    format!(
        "{}/{}/{}",
        endpoint.trim_end_matches('/'),
        bucket.trim_matches('/'),
        object_key.trim_start_matches('/')
    )
}

impl From<MediaAssetRow> for MediaAssetResponse {
    fn from(row: MediaAssetRow) -> Self {
        Self {
            id: row.id,
            bucket: row.bucket,
            object_key: row.object_key,
            original_file_name: row.original_file_name,
            mime_type: row.mime_type,
            size_bytes: row.size_bytes,
            checksum: row.checksum,
            visibility: row.visibility,
            status: row.status,
            storage_provider: row.storage_provider,
            upload_method: row.upload_method,
            upload_expires_at: row.upload_expires_at,
            uploaded_at: row.uploaded_at,
            public_url: row.public_url,
            metadata: row.metadata,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<MediaLinkRow> for MediaLinkResponse {
    fn from(row: MediaLinkRow) -> Self {
        Self {
            id: row.id,
            media_asset_id: row.media_asset_id,
            entity_type: row.entity_type,
            entity_id: row.entity_id,
            purpose: row.purpose,
            created_at: row.created_at,
        }
    }
}
