use axum::{routing::get, Json, Router};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
struct ProofScopeResponse {
    current_phase: &'static str,
    implemented_now: Vec<&'static str>,
    after_backend: Vec<&'static str>,
    after_grant: Vec<&'static str>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/scope", get(scope))
}

async fn scope() -> Json<ProofScopeResponse> {
    Json(ProofScopeResponse {
        current_phase: "backend-foundation",
        implemented_now: vec![
            "proof-family-model",
            "passport-proof-foundation",
            "core-lab-leveling",
        ],
        after_backend: vec![
            "signed-readiness-events",
            "passport-credential-records",
            "media-backed-evidence-bundles",
        ],
        after_grant: vec![
            "cairo-passport-registry",
            "starknet-anchor",
            "nft-or-non-transferable-badge",
            "public-verifier",
        ],
    })
}
