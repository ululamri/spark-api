use axum::{routing::get, Json, Router};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
struct ScopeResponse {
    module: &'static str,
    phase: &'static str,
    implemented_now: Vec<&'static str>,
    next_backend_steps: Vec<&'static str>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/scope", get(scope))
}

async fn scope() -> Json<ScopeResponse> {
    Json(ScopeResponse {
        module: module_path!(),
        phase: "backend-foundation",
        implemented_now: vec!["route-boundary", "api-contract-placeholder"],
        next_backend_steps: vec!["schema", "repository", "service", "authenticated-endpoints"],
    })
}
