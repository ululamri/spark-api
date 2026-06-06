use axum::{routing::get, Json, Router};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
struct HealthResponse {
    ok: bool,
    service: &'static str,
    checked_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/live", get(live))
        .route("/ready", get(ready))
}

async fn live() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        service: "spark-api",
        checked_at: Utc::now(),
    })
}

async fn ready() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        service: "spark-api",
        checked_at: Utc::now(),
    })
}
