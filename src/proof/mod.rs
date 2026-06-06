pub mod ledger;

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{auth::session::require_current_user, error::ApiError, state::AppState};

#[derive(Serialize)]
struct ProofScopeResponse {
    current_phase: &'static str,
    implemented_now: Vec<&'static str>,
    backend_outputs: Vec<&'static str>,
    after_grant: Vec<&'static str>,
}

#[derive(Debug, Deserialize)]
struct ListProofEventsQuery {
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ListResponse<T> {
    items: Vec<T>,
}

#[derive(Debug, Serialize)]
struct ProofEventResponse {
    id: Uuid,
    event_type: String,
    subject_type: String,
    subject_id: String,
    level: Option<String>,
    track: Option<String>,
    source_table: Option<String>,
    source_id: Option<Uuid>,
    event_hash: Option<String>,
    previous_event_hash: Option<String>,
    evidence_root: Option<String>,
    issuer: String,
    schema_version: String,
    payload: Value,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct ProofEventRow {
    id: Uuid,
    event_type: String,
    subject_type: String,
    subject_id: String,
    level: Option<String>,
    track: Option<String>,
    source_table: Option<String>,
    source_id: Option<Uuid>,
    event_hash: Option<String>,
    previous_event_hash: Option<String>,
    evidence_root: Option<String>,
    issuer: String,
    schema_version: String,
    payload: Value,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct EvidenceRootResponse {
    user_id: Uuid,
    event_count: i64,
    evidence_root: Option<String>,
    schema_version: &'static str,
    ready_for_starknet_anchor: bool,
    note: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/scope", get(scope))
        .route("/me/events", get(list_my_events))
        .route("/me/evidence-root", get(my_evidence_root))
}

async fn scope() -> Json<ProofScopeResponse> {
    Json(ProofScopeResponse {
        current_phase: "proof-event-ledger-foundation",
        implemented_now: vec![
            "system-proof-event-ledger",
            "event-hash-chain",
            "evidence-root-read-model",
            "learning-lab-proof-emission",
            "starknet-anchor-prep",
        ],
        backend_outputs: vec![
            "Proof-of-Learning events",
            "Proof-of-Practice events",
            "Proof-of-Safety events",
            "level exam attempt events",
            "checkpoint result events",
        ],
        after_grant: vec![
            "Cairo PassportRegistry",
            "Starknet Sepolia testing",
            "Starknet Mainnet anchor",
            "NFT or non-transferable Passport Badge",
            "public verifier",
        ],
    })
}

async fn list_my_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListProofEventsQuery>,
) -> Result<Json<ListResponse<ProofEventResponse>>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let limit = query.limit.unwrap_or(50).clamp(1, 100);

    let rows = sqlx::query_as::<_, ProofEventRow>(
        r#"
        select id,
               event_type,
               subject_type,
               subject_id,
               level,
               track,
               source_table,
               source_id,
               event_hash,
               previous_event_hash,
               evidence_root,
               issuer,
               schema_version,
               payload,
               created_at
        from proof_events
        where user_id = $1
        order by created_at desc, id desc
        limit $2
        "#,
    )
    .bind(user.id)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(ListResponse {
        items: rows.into_iter().map(ProofEventResponse::from).collect(),
    }))
}

async fn my_evidence_root(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<EvidenceRootResponse>, ApiError> {
    let user = require_current_user(&state, &headers).await?;

    let event_count = sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)
        from proof_events
        where user_id = $1
          and event_hash is not null
        "#,
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;

    let evidence_root = sqlx::query_scalar::<_, String>(
        r#"
        select event_hash
        from proof_events
        where user_id = $1
          and event_hash is not null
        order by created_at desc, id desc
        limit 1
        "#,
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?;

    Ok(Json(EvidenceRootResponse {
        user_id: user.id,
        event_count,
        evidence_root: evidence_root.clone(),
        schema_version: ledger::PROOF_EVENT_SCHEMA_VERSION,
        ready_for_starknet_anchor: evidence_root.is_some(),
        note: "Evidence root is backend-only for now. Starknet anchoring remains grant scope.",
    }))
}

impl From<ProofEventRow> for ProofEventResponse {
    fn from(row: ProofEventRow) -> Self {
        Self {
            id: row.id,
            event_type: row.event_type,
            subject_type: row.subject_type,
            subject_id: row.subject_id,
            level: row.level,
            track: row.track,
            source_table: row.source_table,
            source_id: row.source_id,
            event_hash: row.event_hash,
            previous_event_hash: row.previous_event_hash,
            evidence_root: row.evidence_root,
            issuer: row.issuer,
            schema_version: row.schema_version,
            payload: row.payload,
            created_at: row.created_at,
        }
    }
}
