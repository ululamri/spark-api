use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    error::ApiError,
    media_optimizer::{optimized_public_image_urls, OptimizedMediaUrls},
    state::AppState,
};

#[derive(Debug, Serialize)]
struct OptimizerScopeResponse {
    module: &'static str,
    phase: &'static str,
    enabled: bool,
    public_base_url: String,
    source_base_url: String,
    key_configured: bool,
    salt_configured: bool,
    variants: Vec<&'static str>,
    note: &'static str,
}

#[derive(Debug, Serialize)]
struct PublicAssetOptimizedUrlsResponse {
    asset_id: Uuid,
    mime_type: String,
    public_url: String,
    optimized_urls: Option<OptimizedMediaUrls>,
    checked_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct PublicAssetRow {
    id: Uuid,
    mime_type: String,
    public_url: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/scope", get(scope))
        .route("/public/:asset_id/urls", get(public_asset_urls))
}

async fn scope(State(state): State<AppState>) -> Json<OptimizerScopeResponse> {
    Json(OptimizerScopeResponse {
        module: module_path!(),
        phase: "perf-ux-imgproxy-smoke-api",
        enabled: state.config.media_optimizer_enabled,
        public_base_url: state.config.imgproxy_public_base_url.clone(),
        source_base_url: state.config.imgproxy_source_base_url.clone(),
        key_configured: state.config.imgproxy_key_hex.is_some(),
        salt_configured: state.config.imgproxy_salt_hex.is_some(),
        variants: vec!["avatar_64", "avatar_128", "feed_480", "feed_720", "detail_1080", "detail_1440", "original"],
        note: "This endpoint is for rollout verification. Keep optimizer disabled until imgproxy and Caddy /media/optimized/* are smoke-tested.",
    })
}

async fn public_asset_urls(
    Path(asset_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<PublicAssetOptimizedUrlsResponse>, ApiError> {
    let row = sqlx::query_as::<_, PublicAssetRow>(
        r#"
        select id, mime_type, public_url
        from media_assets
        where id = $1
          and status = 'uploaded'
          and visibility = 'public'
          and moderation_status in ('allowed', 'restored')
        "#,
    )
    .bind(asset_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("public media asset not found".to_string()))?;

    let public_url = row
        .public_url
        .ok_or_else(|| ApiError::BadRequest("public media asset has no public URL".to_string()))?;
    let optimized_urls = optimized_public_image_urls(&state.config, &public_url, &row.mime_type);

    Ok(Json(PublicAssetOptimizedUrlsResponse {
        asset_id: row.id,
        mime_type: row.mime_type,
        public_url,
        optimized_urls,
        checked_at: Utc::now(),
    }))
}
