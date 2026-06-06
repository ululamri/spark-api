use axum::{routing::get, Json, Router};
use serde::Serialize;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::state::AppState;

#[derive(Serialize)]
struct RootResponse {
    service: &'static str,
    phase: &'static str,
    frontend: &'static str,
    backend: &'static str,
    database: &'static str,
    storage: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(root))
        .nest("/health", crate::health::router())
        .nest("/v1/auth", crate::auth::router())
        .nest("/v1/learning", crate::learning::router())
        .nest("/v1/lab", crate::lab::router())
        .nest("/v1/media", crate::media::router())
        .nest("/v1/proof", crate::proof::router())
        .nest("/v1/passport", crate::passport::router())
        .nest("/v1/community", crate::community::router())
        .nest("/v1/hub", crate::hub::router())
        .nest("/v1/social", crate::social::router())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

async fn root() -> Json<RootResponse> {
    Json(RootResponse {
        service: "Karyra Spark API",
        phase: "backend-foundation",
        frontend: "SvelteKit",
        backend: "Rust/Axum",
        database: "PostgreSQL + SQLx",
        storage: "S3-compatible self-hosted storage: MinIO/Garage first",
    })
}
