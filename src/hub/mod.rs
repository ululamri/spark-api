use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    auth::session::require_current_user,
    error::ApiError,
    progress::{clean_required_id, payload_or_empty},
    proof::ledger::{record_system_event, SystemProofEventInput},
    state::AppState,
};

#[derive(Serialize)]
struct ScopeResponse {
    module: &'static str,
    phase: &'static str,
    implemented_now: Vec<&'static str>,
    proof_outputs: Vec<&'static str>,
    next_backend_steps: Vec<&'static str>,
}

#[derive(Debug, Deserialize)]
pub struct HubSignalRequest {
    pub payload: Option<Value>,
}

#[derive(Debug, Serialize)]
struct ListResponse<T> {
    items: Vec<T>,
}

#[derive(Debug, Serialize)]
struct HubResourceSaveResponse {
    id: Uuid,
    resource_id: String,
    status: String,
    saved_at: DateTime<Utc>,
    unsaved_at: Option<DateTime<Utc>>,
    payload: Value,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct HubResourceSaveRow {
    id: Uuid,
    resource_id: String,
    status: String,
    saved_at: DateTime<Utc>,
    unsaved_at: Option<DateTime<Utc>>,
    payload: Value,
    updated_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/scope", get(scope))
        .route("/me/resources", get(list_my_resources))
        .route("/resources/:resource_id/save", post(save_resource))
        .route("/resources/:resource_id/unsave", post(unsave_resource))
}

async fn scope() -> Json<ScopeResponse> {
    Json(ScopeResponse {
        module: module_path!(),
        phase: "hub-exploration-signals",
        implemented_now: vec![
            "authenticated-resource-save-signal",
            "resource-unsave-lifecycle",
            "proof-of-exploration-signal-foundation",
        ],
        proof_outputs: vec!["proof_of_exploration_signal_recorded"],
        next_backend_steps: vec![
            "resource catalog database",
            "risk/readiness gate policy",
            "hub mission completion",
            "ecosystem verifier integration",
        ],
    })
}

async fn list_my_resources(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ListResponse<HubResourceSaveResponse>>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let rows = sqlx::query_as::<_, HubResourceSaveRow>(
        r#"
        select id, resource_id, status, saved_at, unsaved_at, payload, updated_at
        from hub_resource_saves
        where user_id = $1 and status = 'saved'
        order by updated_at desc
        limit 100
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(ListResponse {
        items: rows.into_iter().map(HubResourceSaveResponse::from).collect(),
    }))
}

async fn save_resource(
    Path(resource_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<HubSignalRequest>,
) -> Result<(StatusCode, Json<HubResourceSaveResponse>), ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let resource_id = clean_required_id(&resource_id, "resource_id")?;
    let payload = payload_or_empty(payload.payload);
    let id = Uuid::new_v4();

    let row = sqlx::query_as::<_, HubResourceSaveRow>(
        r#"
        insert into hub_resource_saves (
            id, user_id, resource_id, status, saved_at, unsaved_at, payload
        )
        values ($1, $2, $3, 'saved', now(), null, $4)
        on conflict (user_id, resource_id) do update set
          status = 'saved',
          saved_at = now(),
          unsaved_at = null,
          payload = excluded.payload,
          updated_at = now()
        returning id, resource_id, status, saved_at, unsaved_at, payload, updated_at
        "#,
    )
    .bind(id)
    .bind(user.id)
    .bind(resource_id)
    .bind(payload)
    .fetch_one(&state.db)
    .await?;

    let _proof_event = record_system_event(
        &state,
        SystemProofEventInput {
            user_id: user.id,
            event_type: "proof_of_exploration_signal_recorded".to_string(),
            subject_type: "hub_resource".to_string(),
            subject_id: row.resource_id.clone(),
            level: None,
            track: Some("hub".to_string()),
            source_table: "hub_resource_saves".to_string(),
            source_id: row.id,
            payload: json!({
                "resource_id": row.resource_id.clone(),
                "status": row.status.clone(),
                "source": "hub.resource_save"
            }),
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(HubResourceSaveResponse::from(row))))
}

async fn unsave_resource(
    Path(resource_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<HubSignalRequest>,
) -> Result<Json<HubResourceSaveResponse>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let resource_id = clean_required_id(&resource_id, "resource_id")?;
    let payload = payload_or_empty(payload.payload);

    let row = sqlx::query_as::<_, HubResourceSaveRow>(
        r#"
        update hub_resource_saves
        set status = 'unsaved', unsaved_at = now(), payload = $3, updated_at = now()
        where user_id = $1 and resource_id = $2
        returning id, resource_id, status, saved_at, unsaved_at, payload, updated_at
        "#,
    )
    .bind(user.id)
    .bind(resource_id)
    .bind(payload)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("hub resource save not found".to_string()))?;

    Ok(Json(HubResourceSaveResponse::from(row)))
}

impl From<HubResourceSaveRow> for HubResourceSaveResponse {
    fn from(row: HubResourceSaveRow) -> Self {
        Self {
            id: row.id,
            resource_id: row.resource_id,
            status: row.status,
            saved_at: row.saved_at,
            unsaved_at: row.unsaved_at,
            payload: row.payload,
            updated_at: row.updated_at,
        }
    }
}
