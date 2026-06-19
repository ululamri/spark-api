#!/usr/bin/env node
import { readFileSync, writeFileSync } from 'node:fs';

const file = 'src/media/mod.rs';
let source = readFileSync(file, 'utf8');

const oldComplete = `async fn complete_upload(
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
`;

const newComplete = `async fn complete_upload(
    Path(asset_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CompleteUploadRequest>,
) -> Result<Json<MediaAssetResponse>, ApiError> {
    let user = require_current_user(&state, &headers).await?;

    if payload.checksum.is_some() || payload.size_bytes.is_some() {
        return Err(ApiError::BadRequest(
            "checksum and size_bytes are controlled by the upload intent and are not accepted during completion".to_string(),
        ));
    }

    let existing = fetch_owned_asset(&state, user.id, asset_id).await?;
    if existing.status == "uploaded" {
        return Ok(Json(MediaAssetResponse::from(existing)));
    }

    if let Some(expires_at) = existing.upload_expires_at {
        if expires_at < Utc::now() {
            return Err(ApiError::BadRequest("media upload intent has expired".to_string()));
        }
    }

    verify_uploaded_object(&state, &existing).await?;
    let metadata = payload.metadata;

    let row = sqlx::query_as::<_, MediaAssetRow>(
        r#"
        update media_assets
        set status = 'uploaded',
            uploaded_at = coalesce(uploaded_at, now()),
            metadata = coalesce($3, metadata),
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
    .bind(metadata)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("media asset not found".to_string()))?;

    Ok(Json(MediaAssetResponse::from(row)))
}
`;

const helperAnchor = `async fn fetch_owned_asset(
    state: &AppState,
    user_id: Uuid,
    asset_id: Uuid,
) -> Result<MediaAssetRow, ApiError> {`;

const helper = `async fn verify_uploaded_object(state: &AppState, asset: &MediaAssetRow) -> Result<(), ApiError> {
    let url = storage_access_url(state, "HEAD", &asset.bucket, &asset.object_key)?;
    let response = reqwest::Client::new()
        .head(url)
        .send()
        .await
        .map_err(|error| {
            tracing::warn!(?error, asset_id = %asset.id, "media object HEAD verification failed");
            ApiError::ServiceUnavailable("media object could not be verified in storage".to_string())
        })?;

    if !response.status().is_success() {
        tracing::warn!(status = %response.status(), asset_id = %asset.id, "media object HEAD verification returned non-success status");
        return Err(ApiError::BadRequest("uploaded media object was not found in storage".to_string()));
    }

    Ok(())
}

`;

if (!source.includes(oldComplete)) {
  throw new Error('Expected complete_upload block was not found. Refusing to patch.');
}
source = source.replace(oldComplete, newComplete);

if (!source.includes('async fn verify_uploaded_object(')) {
  if (!source.includes(helperAnchor)) {
    throw new Error('Expected fetch_owned_asset anchor was not found. Refusing to insert helper.');
  }
  source = source.replace(helperAnchor, helper + helperAnchor);
}

writeFileSync(file, source);
console.log('PASS 20B media completion hardening patch applied.');
