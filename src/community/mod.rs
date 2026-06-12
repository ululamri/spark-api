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
pub struct CommunitySignalRequest {
    pub payload: Option<Value>,
}

#[derive(Debug, Serialize)]
struct ListResponse<T> {
    items: Vec<T>,
}

#[derive(Debug, Serialize)]
struct WorkshopRegistrationResponse {
    id: Uuid,
    workshop_id: String,
    status: String,
    registered_at: DateTime<Utc>,
    canceled_at: Option<DateTime<Utc>>,
    payload: Value,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct WorkshopRegistrationRow {
    id: Uuid,
    workshop_id: String,
    status: String,
    registered_at: DateTime<Utc>,
    canceled_at: Option<DateTime<Utc>>,
    payload: Value,
    updated_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/scope", get(scope))
        .route("/me/workshops", get(list_my_workshops))
        .route("/workshops/:workshop_id/register", post(register_workshop))
        .route("/workshops/:workshop_id/cancel", post(cancel_workshop))
}

async fn scope() -> Json<ScopeResponse> {
    Json(ScopeResponse {
        module: module_path!(),
        phase: "community-participation-signals",
        implemented_now: vec![
            "authenticated-workshop-registration-signal",
            "registration-cancel-lifecycle",
            "proof-of-participation-signal-foundation",
        ],
        proof_outputs: vec!["proof_of_participation_signal_recorded"],
        next_backend_steps: vec![
            "real event capacity",
            "community verifier workflow",
            "attendance verification",
            "participation tier policy",
        ],
    })
}

async fn list_my_workshops(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ListResponse<WorkshopRegistrationResponse>>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let rows = sqlx::query_as::<_, WorkshopRegistrationRow>(
        r#"
        select id, workshop_id, status, registered_at, canceled_at, payload, updated_at
        from community_workshop_registrations
        where user_id = $1 and status = 'registered'
        order by updated_at desc
        limit 100
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(ListResponse {
        items: rows
            .into_iter()
            .map(WorkshopRegistrationResponse::from)
            .collect(),
    }))
}

async fn register_workshop(
    Path(workshop_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CommunitySignalRequest>,
) -> Result<(StatusCode, Json<WorkshopRegistrationResponse>), ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let workshop_id = clean_required_id(&workshop_id, "workshop_id")?;
    let payload = payload_or_empty(payload.payload);
    let id = Uuid::new_v4();

    let row = sqlx::query_as::<_, WorkshopRegistrationRow>(
        r#"
        insert into community_workshop_registrations (
            id, user_id, workshop_id, status, registered_at, canceled_at, payload
        )
        values ($1, $2, $3, 'registered', now(), null, $4)
        on conflict (user_id, workshop_id) do update set
          status = 'registered',
          registered_at = now(),
          canceled_at = null,
          payload = excluded.payload,
          updated_at = now()
        returning id, workshop_id, status, registered_at, canceled_at, payload, updated_at
        "#,
    )
    .bind(id)
    .bind(user.id)
    .bind(workshop_id)
    .bind(payload)
    .fetch_one(&state.db)
    .await?;

    let _proof_event = record_system_event(
        &state,
        SystemProofEventInput {
            user_id: user.id,
            event_type: "proof_of_participation_signal_recorded".to_string(),
            subject_type: "workshop".to_string(),
            subject_id: row.workshop_id.clone(),
            level: None,
            track: Some("community".to_string()),
            source_table: "community_workshop_registrations".to_string(),
            source_id: row.id,
            payload: json!({
                "workshop_id": row.workshop_id.clone(),
                "status": row.status.clone(),
                "source": "community.workshop_registration"
            }),
        },
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(WorkshopRegistrationResponse::from(row)),
    ))
}

async fn cancel_workshop(
    Path(workshop_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CommunitySignalRequest>,
) -> Result<Json<WorkshopRegistrationResponse>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let workshop_id = clean_required_id(&workshop_id, "workshop_id")?;
    let payload = payload_or_empty(payload.payload);

    let row = sqlx::query_as::<_, WorkshopRegistrationRow>(
        r#"
        update community_workshop_registrations
        set status = 'canceled', canceled_at = now(), payload = $3, updated_at = now()
        where user_id = $1 and workshop_id = $2
        returning id, workshop_id, status, registered_at, canceled_at, payload, updated_at
        "#,
    )
    .bind(user.id)
    .bind(workshop_id)
    .bind(payload)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("workshop registration not found".to_string()))?;

    Ok(Json(WorkshopRegistrationResponse::from(row)))
}

impl From<WorkshopRegistrationRow> for WorkshopRegistrationResponse {
    fn from(row: WorkshopRegistrationRow) -> Self {
        Self {
            id: row.id,
            workshop_id: row.workshop_id,
            status: row.status,
            registered_at: row.registered_at,
            canceled_at: row.canceled_at,
            payload: row.payload,
            updated_at: row.updated_at,
        }
    }
}
