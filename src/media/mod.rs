use axum::{extract::State, routing::get, routing::post, Json, Router};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

const MAX_UPLOAD_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Serialize)]
struct MediaPolicyResponse {
    provider: &'static str,
    storage_default: &'static str,
    production_small: &'static str,
    future_scale: &'static str,
    max_upload_bytes: u64,
    allowed_mime_prefixes: Vec<&'static str>,
    buckets: Vec<&'static str>,
}

#[derive(Debug, Deserialize)]
pub struct UploadIntentRequest {
    pub purpose: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub private: Option<bool>,
}

#[derive(Serialize)]
struct UploadIntentResponse {
    upload_id: String,
    provider: &'static str,
    bucket: String,
    object_key: String,
    upload_method: &'static str,
    upload_url: String,
    expires_at: DateTime<Utc>,
    note: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/policy", get(policy))
        .route("/upload-intents", post(create_upload_intent))
}

async fn policy() -> Json<MediaPolicyResponse> {
    Json(MediaPolicyResponse {
        provider: "s3-compatible",
        storage_default: "MinIO for local development",
        production_small: "Garage or MinIO self-hosted",
        future_scale: "Cloudflare R2/S3-compatible provider when needed",
        max_upload_bytes: MAX_UPLOAD_BYTES,
        allowed_mime_prefixes: vec!["image/", "application/pdf", "text/"],
        buckets: vec![
            "spark-public",
            "spark-private",
            "avatars",
            "lesson-media",
            "community-media",
            "event-media",
            "passport-assets",
            "private-evidence",
        ],
    })
}

async fn create_upload_intent(
    State(state): State<AppState>,
    Json(payload): Json<UploadIntentRequest>,
) -> Result<Json<UploadIntentResponse>, ApiError> {
    validate_upload_request(&payload)?;

    let upload_id = Uuid::new_v4();
    let purpose = sanitize_object_segment(&payload.purpose, "general");
    let file_name = sanitize_object_segment(&payload.file_name, "upload.bin");
    let object_key = format!("{purpose}/{upload_id}/{file_name}");
    let bucket = if payload.private.unwrap_or(false) {
        state.config.s3_bucket_private.clone()
    } else {
        state.config.s3_bucket_public.clone()
    };

    Ok(Json(UploadIntentResponse {
        upload_id: upload_id.to_string(),
        provider: "s3-compatible",
        bucket: bucket.clone(),
        object_key: object_key.clone(),
        upload_method: "PUT",
        upload_url: format!("{}/{}/{}", state.config.s3_endpoint, bucket, object_key),
        expires_at: Utc::now() + Duration::minutes(15),
        note: "Foundation placeholder. Production will return a signed S3-compatible upload URL.",
    }))
}

fn validate_upload_request(payload: &UploadIntentRequest) -> Result<(), ApiError> {
    if payload.purpose.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "upload purpose is required".to_string(),
        ));
    }

    if payload.file_name.trim().is_empty() {
        return Err(ApiError::BadRequest("file name is required".to_string()));
    }

    if payload.mime_type.trim().is_empty() {
        return Err(ApiError::BadRequest("mime type is required".to_string()));
    }

    if payload.size_bytes == 0 {
        return Err(ApiError::BadRequest(
            "file size must be greater than zero".to_string(),
        ));
    }

    if payload.size_bytes > MAX_UPLOAD_BYTES {
        return Err(ApiError::BadRequest(format!(
            "file size exceeds MVP limit of {MAX_UPLOAD_BYTES} bytes"
        )));
    }

    Ok(())
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
