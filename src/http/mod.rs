use axum::{
    http::{header, HeaderValue, Method},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{config::AppConfig, state::AppState};

#[derive(Serialize)]
struct RootResponse {
    service: &'static str,
    phase: &'static str,
    frontend: &'static str,
    backend: &'static str,
    database: &'static str,
    storage: &'static str,
    auth: &'static str,
    progress: &'static str,
}

pub fn router(config: &AppConfig) -> Router<AppState> {
    Router::new()
        .route("/", get(root))
        .nest("/health", crate::health::router())
        .nest("/v1/auth", crate::auth::router())
        .nest("/v1/profile", crate::profile::router())
        .nest("/v1/learning", crate::learning::router())
        .nest("/v1/lab", crate::lab::router())
        .nest("/v1/media", crate::media::router())
        .nest("/v1/media-optimizer", crate::media_optimizer_api::router())
        .nest("/v1/proof", crate::proof::router())
        .nest("/v1/passport", crate::passport::router())
        .nest("/v1/community", crate::community::router())
        .nest("/v1/hub", crate::hub::router())
        .nest("/v1/social", crate::social::router())
        .nest("/v1/ai", crate::ai::router())
        .nest("/api/admin/auth", crate::admin_login::router())
        .nest("/api/admin/onboarding", crate::admin_onboarding::router())
        .nest("/api/admin/ai", crate::admin_ai::router())
        .nest("/api/admin/audit", crate::admin_audit::router())
        .nest("/api/admin/cms", crate::admin_cms::router())
        .nest("/api/admin/team", crate::admin_team::router())
        .nest("/api/admin/social/bulk", crate::admin_social_bulk::router())
        .nest("/api/admin/social/ml", crate::admin_social_ml::router())
        .nest("/api/admin/social/ops", crate::admin_social_ops::router())
        .nest("/api/admin/social", crate::admin_social::router())
        .nest("/api/admin", crate::admin::router())
        .layer(cors_layer(config))
        .layer(TraceLayer::new_for_http())
}

async fn root() -> Json<RootResponse> {
    Json(RootResponse {
        service: "Karyra Spark API",
        phase: "learning-lab-progress-api",
        frontend: "SvelteKit",
        backend: "Rust/Axum",
        database: "PostgreSQL + SQLx",
        storage: "S3-compatible self-hosted storage: MinIO/Garage first",
        auth: "httpOnly cookie session + system proof ledger",
        progress: "authenticated Core/Learn and Lab progress records",
    })
}

fn cors_layer(config: &AppConfig) -> CorsLayer {
    let origin = HeaderValue::from_str(&config.web_origin)
        .unwrap_or_else(|_| HeaderValue::from_static("http://127.0.0.1:5173"));

    CorsLayer::new()
        .allow_origin(origin)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            header::HeaderName::from_static("x-karyra-admin-token"),
        ])
        .allow_credentials(true)
}
