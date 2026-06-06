use axum::{extract::State, routing::get, Json, Router};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{error::ApiError, state::AppState};

#[derive(Serialize)]
struct HealthResponse {
    ok: bool,
    service: &'static str,
    checked_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct ReadyResponse {
    ok: bool,
    service: &'static str,
    database: &'static str,
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

async fn ready(State(state): State<AppState>) -> Result<Json<ReadyResponse>, ApiError> {
    let database_ok = sqlx::query_scalar::<_, i32>("select 1")
        .fetch_one(&state.db)
        .await
        .is_ok();

    if !database_ok {
        return Err(ApiError::ServiceUnavailable(
            "database is not reachable".to_string(),
        ));
    }

    Ok(Json(ReadyResponse {
        ok: true,
        service: "spark-api",
        database: "reachable",
        checked_at: Utc::now(),
    }))
}
