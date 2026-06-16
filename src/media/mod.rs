use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Redirect,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{auth::session::require_current_user, error::ApiError, state::AppState};

const MAX_UPLOAD_BYTES: i64 = 8 * 1024 * 1024;
const UPLOAD_TTL_MINUTES: i64 = 15;
const STORAGE_PROVIDER: &str = "s3-compatible";
const UPLOAD_METHOD: &str = "PUT";
const PUBLIC_MEDIA_BASE_PATH: &str = "/v1/media/public";

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
    public_delivery_path: &'static str,
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
    presigned: bool,
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
        .route("/public/:asset_id", get(public_asset_redirect))
        .route("/me/assets", get(list_my_assets))
        .route("/upload-intents", post(create_upload_intent))
        .route("/assets/:asset_id", get(get_my_asset))
        .route("/assets/:asset_id/complete", post(complete_upload))
        .route("/assets/:asset_id/links", post(create_media_link))
}

async fn policy(State(state): State<AppState>) -> Json<MediaPolicyResponse> {
    Json(MediaPolicyResponse {
        provider: STORAGE_PROVIDER,
        current_phase: "presigned-media-runtime",
        storage_default: "MinIO for local development and small production",
        production_small: "Garage or MinIO self-hosted with presigned PUT/GET URLs",
        future_scale: "Cloudflare R2/S3-compatible provider when needed",
        max_upload_bytes: MAX_UPLOAD_BYTES,
        upload_ttl_minutes: effective_presign_seconds(&state) / 60,
        allowed_mime_prefixes: vec!["image/", "application/pdf", "text/"],
        accepted_purposes: accepted_purposes(),
        physical_buckets: vec!["spark-public", "spark-private"],
        public_delivery_path: PUBLIC_MEDIA_BASE_PATH,
        note: "Upload intents now return presigned S3-compatible URLs when S3/MINIO credentials are configured. Public media is delivered through a backend redirect path instead of exposing raw bucket URLs in feed payloads.",
    })
}

async fn list_my_assets(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ListResponse<MediaAssetResponse>>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let rows = sqlx::query_as::<_, MediaAssetRow>(MEDIA_ASSET_SELECT_OWNED)
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

async fn public_asset_redirect(
    Path(asset_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Redirect, ApiError> {
    let row = sqlx::query_as::<_, MediaAssetRow>(MEDIA_ASSET_SELECT_PUBLIC)
        .bind(asset_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::BadRequest("public media asset not found".to_string()))?;

    let target = storage_access_url(&state, "GET", &row.bucket, &row.object_key)?;
    Ok(Redirect::temporary(&target))
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
    let upload_url = storage_access_url(&state, UPLOAD_METHOD, &bucket, &object_key)?;
    let upload_expires_at = Utc::now() + Duration::seconds(effective_presign_seconds(&state));
    let public_url = if visibility == "public" {
        Some(format!("{PUBLIC_MEDIA_BASE_PATH}/{asset_id}"))
    } else {
        None
    };
    let metadata = json!({
        "purpose": purpose,
        "entity_type": entity_type,
        "entity_id": entity_id,
        "client_metadata": payload.metadata.unwrap_or_else(|| json!({})),
        "source": "media.presigned_upload_intent",
        "delivery": if visibility == "public" { "backend_redirect" } else { "private_asset" }
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
    .bind(upload_expires_at)
    .bind(public_url)
    .bind(metadata)
    .fetch_one(&state.db)
    .await?;

    let response = UploadIntentResponse {
        provider: STORAGE_PROVIDER,
        upload_method: UPLOAD_METHOD,
        upload_url,
        expires_at: upload_expires_at,
        presigned: has_s3_credentials(&state),
        asset: MediaAssetResponse::from(row),
        note: "Upload intent stored. When S3/MINIO credentials are configured, upload_url is a presigned PUT URL. Complete the asset after upload with /v1/media/assets/:asset_id/complete.",
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
    let row = sqlx::query_as::<_, MediaAssetRow>(MEDIA_ASSET_SELECT_ONE_OWNED)
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

fn has_s3_credentials(state: &AppState) -> bool {
    state.config.s3_access_key.is_some() && state.config.s3_secret_key.is_some()
}

fn effective_presign_seconds(state: &AppState) -> i64 {
    state.config.s3_presign_expires_seconds.clamp(60, 604_800)
}

fn storage_access_url(
    state: &AppState,
    method: &str,
    bucket: &str,
    object_key: &str,
) -> Result<String, ApiError> {
    let Some(access_key) = state.config.s3_access_key.as_deref() else {
        return Ok(storage_url(&state.config.s3_endpoint, bucket, object_key));
    };
    let Some(secret_key) = state.config.s3_secret_key.as_deref() else {
        return Ok(storage_url(&state.config.s3_endpoint, bucket, object_key));
    };

    presign_s3_url(
        &state.config.s3_endpoint,
        bucket,
        object_key,
        method,
        &state.config.s3_region,
        access_key,
        secret_key,
        effective_presign_seconds(state) as u32,
    )
}

#[derive(Debug)]
struct S3EndpointParts {
    scheme: String,
    host: String,
    path_prefix: String,
}

fn parse_s3_endpoint(endpoint: &str) -> Result<S3EndpointParts, ApiError> {
    let endpoint = endpoint.trim().trim_end_matches('/');
    let (scheme, rest) = if let Some(rest) = endpoint.strip_prefix("https://") {
        ("https", rest)
    } else if let Some(rest) = endpoint.strip_prefix("http://") {
        ("http", rest)
    } else {
        return Err(ApiError::ServiceUnavailable(
            "S3_ENDPOINT must start with http:// or https://".to_string(),
        ));
    };

    let (host, path_prefix) = rest.split_once('/').unwrap_or((rest, ""));
    if host.trim().is_empty() {
        return Err(ApiError::ServiceUnavailable(
            "S3_ENDPOINT host is empty".to_string(),
        ));
    }

    Ok(S3EndpointParts {
        scheme: scheme.to_string(),
        host: host.to_string(),
        path_prefix: path_prefix.trim_matches('/').to_string(),
    })
}

fn presign_s3_url(
    endpoint: &str,
    bucket: &str,
    object_key: &str,
    method: &str,
    region: &str,
    access_key: &str,
    secret_key: &str,
    expires_seconds: u32,
) -> Result<String, ApiError> {
    let endpoint = parse_s3_endpoint(endpoint)?;
    let now = Utc::now();
    let date = now.format("%Y%m%d").to_string();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let region = if region.trim().is_empty() {
        "us-east-1"
    } else {
        region.trim()
    };
    let credential_scope = format!("{date}/{region}/s3/aws4_request");
    let credential = format!("{access_key}/{credential_scope}");
    let canonical_uri = canonical_storage_uri(&endpoint.path_prefix, bucket, object_key);
    let mut query = vec![
        (
            "X-Amz-Algorithm".to_string(),
            "AWS4-HMAC-SHA256".to_string(),
        ),
        ("X-Amz-Credential".to_string(), credential),
        ("X-Amz-Date".to_string(), amz_date),
        ("X-Amz-Expires".to_string(), expires_seconds.to_string()),
        ("X-Amz-SignedHeaders".to_string(), "host".to_string()),
    ];
    query.sort_by(|left, right| left.0.cmp(&right.0));
    let canonical_query = canonical_query_string(&query);
    let canonical_headers = format!("host:{}\n", endpoint.host);
    let canonical_request = format!(
        "{}\n{}\n{}\n{}\nhost\nUNSIGNED-PAYLOAD",
        method.to_ascii_uppercase(),
        canonical_uri,
        canonical_query,
        canonical_headers
    );
    let canonical_request_hash = sha256_hex(canonical_request.as_bytes());
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        query
            .iter()
            .find(|(key, _)| key == "X-Amz-Date")
            .map(|(_, value)| value.as_str())
            .unwrap_or_default(),
        credential_scope,
        canonical_request_hash
    );
    let signing_key = s3_signing_key(secret_key, &date, region);
    let signature = hex_lower(&hmac_sha256(&signing_key, string_to_sign.as_bytes()));

    Ok(format!(
        "{}://{}{}?{}&X-Amz-Signature={}",
        endpoint.scheme, endpoint.host, canonical_uri, canonical_query, signature
    ))
}

fn canonical_storage_uri(path_prefix: &str, bucket: &str, object_key: &str) -> String {
    let mut segments = Vec::new();

    for segment in path_prefix.split('/') {
        if !segment.is_empty() {
            segments.push(segment);
        }
    }
    for segment in bucket.trim_matches('/').split('/') {
        if !segment.is_empty() {
            segments.push(segment);
        }
    }
    for segment in object_key.trim_matches('/').split('/') {
        if !segment.is_empty() {
            segments.push(segment);
        }
    }

    format!(
        "/{}",
        segments
            .into_iter()
            .map(percent_encode)
            .collect::<Vec<_>>()
            .join("/")
    )
}

fn canonical_query_string(query: &[(String, String)]) -> String {
    query
        .iter()
        .map(|(key, value)| format!("{}={}", percent_encode(key), percent_encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn percent_encode(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for byte in input.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(*byte as char)
            }
            _ => output.push_str(&format!("%{:02X}", byte)),
        }
    }
    output
}

fn s3_signing_key(secret_key: &str, date: &str, region: &str) -> [u8; 32] {
    let date_key = hmac_sha256(format!("AWS4{secret_key}").as_bytes(), date.as_bytes());
    let region_key = hmac_sha256(&date_key, region.as_bytes());
    let service_key = hmac_sha256(&region_key, b"s3");
    hmac_sha256(&service_key, b"aws4_request")
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    const BLOCK_SIZE: usize = 64;
    let mut normalized_key = [0u8; BLOCK_SIZE];

    if key.len() > BLOCK_SIZE {
        let hashed = Sha256::digest(key);
        normalized_key[..32].copy_from_slice(&hashed);
    } else {
        normalized_key[..key.len()].copy_from_slice(key);
    }

    let mut outer_key_pad = [0x5cu8; BLOCK_SIZE];
    let mut inner_key_pad = [0x36u8; BLOCK_SIZE];
    for index in 0..BLOCK_SIZE {
        outer_key_pad[index] ^= normalized_key[index];
        inner_key_pad[index] ^= normalized_key[index];
    }

    let mut inner = Sha256::new();
    inner.update(inner_key_pad);
    inner.update(data);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(outer_key_pad);
    outer.update(inner_hash);
    let output = outer.finalize();

    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&output);
    bytes
}

fn sha256_hex(input: &[u8]) -> String {
    hex_lower(&Sha256::digest(input))
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
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

const MEDIA_ASSET_SELECT_OWNED: &str = r#"
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
"#;

const MEDIA_ASSET_SELECT_ONE_OWNED: &str = r#"
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
"#;

const MEDIA_ASSET_SELECT_PUBLIC: &str = r#"
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
where id = $1 and status = 'uploaded' and visibility = 'public'
"#;
