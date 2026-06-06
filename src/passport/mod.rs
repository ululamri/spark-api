use axum::{routing::get, Json, Router};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
struct PassportScopeResponse {
    product: &'static str,
    proof_output: &'static str,
    backend_role: Vec<&'static str>,
    starknet_grant_scope: Vec<&'static str>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/scope", get(scope))
}

async fn scope() -> Json<PassportScopeResponse> {
    Json(PassportScopeResponse {
        product: "Passport Spark",
        proof_output: "Proof-of-Readiness",
        backend_role: vec![
            "eligibility-engine",
            "issuer-policy",
            "credential-lifecycle",
            "evidence-root-prep",
        ],
        starknet_grant_scope: vec![
            "PassportRegistry",
            "Starknet Mainnet anchor",
            "NFT-ready badge",
            "verifier page",
        ],
    })
}
