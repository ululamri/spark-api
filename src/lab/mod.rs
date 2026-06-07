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
    progress::{
        clean_required_id, is_final_attempt_status, normalize_attempt_status, normalize_level,
        passed_from_score, payload_or_empty, validate_optional_score, validate_score,
    },
    proof::ledger::{record_system_event, SystemProofEventInput},
    state::AppState,
};

#[derive(Serialize)]
struct ScopeResponse {
    module: &'static str,
    phase: &'static str,
    implemented_now: Vec<&'static str>,
    next_backend_steps: Vec<&'static str>,
}

#[derive(Debug, Deserialize)]
pub struct LabAttemptRequest {
    pub lab_id: String,
    pub level: String,
    pub status: Option<String>,
    pub score: Option<i32>,
    pub safety_score: Option<i32>,
    pub payload: Option<Value>,
}

#[derive(Debug, Serialize)]
struct LabAttemptResponse {
    id: Uuid,
    lab_id: String,
    level: String,
    status: String,
    score: Option<i32>,
    safety_score: Option<i32>,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct LabAttemptRow {
    id: Uuid,
    lab_id: String,
    level: String,
    status: String,
    score: Option<i32>,
    safety_score: Option<i32>,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct LabCheckpointResultRequest {
    pub lab_id: String,
    pub level: String,
    pub score: i32,
    pub passed: Option<bool>,
    pub payload: Option<Value>,
}

#[derive(Debug, Serialize)]
struct CheckpointResultResponse {
    id: Uuid,
    track: String,
    subject_id: String,
    checkpoint_id: String,
    level: String,
    score: i32,
    passed: bool,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct CheckpointResultRow {
    id: Uuid,
    track: String,
    subject_id: String,
    checkpoint_id: String,
    level: String,
    score: i32,
    passed: bool,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct LabExamAttemptRequest {
    pub level: String,
    pub exam_id: String,
    pub score: i32,
    pub passed: Option<bool>,
    pub exam_version: Option<String>,
    pub payload: Option<Value>,
}

#[derive(Debug, Serialize)]
struct ExamAttemptResponse {
    id: Uuid,
    track: String,
    level: String,
    exam_id: String,
    score: i32,
    passed: bool,
    attempt_number: i32,
    exam_version: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct ExamAttemptRow {
    id: Uuid,
    track: String,
    level: String,
    exam_id: String,
    score: i32,
    passed: bool,
    attempt_number: i32,
    exam_version: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ListResponse<T> {
    items: Vec<T>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/scope", get(scope))
        .route("/me/attempts", get(list_my_lab_attempts))
        .route("/attempts", post(create_lab_attempt))
        .route(
            "/checkpoints/:checkpoint_id/results",
            post(record_lab_checkpoint_result),
        )
        .route("/exam-attempts", post(record_lab_exam_attempt))
}

async fn scope() -> Json<ScopeResponse> {
    Json(ScopeResponse {
        module: module_path!(),
        phase: "lab-progress-api+proof-ledger",
        implemented_now: vec![
            "authenticated-lab-attempts",
            "proof-of-practice-recording",
            "proof-of-safety-score-foundation",
            "lab-level-exam-attempt-recording",
        ],
        next_backend_steps: vec![
            "passport-eligibility-aggregation",
            "lab-simulation-policy",
            "safety-checklist-versioning",
            "dojo-provable-lab-roadmap",
        ],
    })
}

async fn list_my_lab_attempts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ListResponse<LabAttemptResponse>>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let rows = sqlx::query_as::<_, LabAttemptRow>(
        r#"
        select id, lab_id, level, status, score, safety_score, started_at, completed_at, updated_at
        from lab_attempts
        where user_id = $1
        order by updated_at desc
        limit 100
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(ListResponse {
        items: rows.into_iter().map(LabAttemptResponse::from).collect(),
    }))
}

async fn create_lab_attempt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LabAttemptRequest>,
) -> Result<(StatusCode, Json<LabAttemptResponse>), ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let lab_id = clean_required_id(&payload.lab_id, "lab_id")?;
    let level = normalize_level(&payload.level)?;
    let status = normalize_attempt_status(payload.status.as_deref())?;
    let score = validate_optional_score(payload.score, "score")?;
    let safety_score = validate_optional_score(payload.safety_score, "safety_score")?;
    let completed_at = if is_final_attempt_status(&status) {
        Some(Utc::now())
    } else {
        None
    };
    let payload = payload_or_empty(payload.payload);

    let row = sqlx::query_as::<_, LabAttemptRow>(
        r#"
        insert into lab_attempts (
            user_id, lab_id, level, status, score, safety_score, completed_at, payload
        )
        values ($1, $2, $3, $4, $5, $6, $7, $8)
        returning id, lab_id, level, status, score, safety_score, started_at, completed_at, updated_at
        "#,
    )
    .bind(user.id)
    .bind(lab_id)
    .bind(level)
    .bind(status)
    .bind(score)
    .bind(safety_score)
    .bind(completed_at)
    .bind(payload)
    .fetch_one(&state.db)
    .await?;

    let _proof_event = record_system_event(
        &state,
        SystemProofEventInput {
            user_id: user.id,
            event_type: "proof_of_practice_lab_attempt_recorded".to_string(),
            subject_type: "lab".to_string(),
            subject_id: row.lab_id.clone(),
            level: Some(row.level.clone()),
            track: Some("lab".to_string()),
            source_table: "lab_attempts".to_string(),
            source_id: row.id,
            payload: json!({
                "lab_id": row.lab_id.clone(),
                "level": row.level.clone(),
                "status": row.status.clone(),
                "score": row.score,
                "safety_score": row.safety_score,
                "completed_at": row.completed_at.clone(),
                "source": "lab.lab_attempts"
            }),
        },
    )
    .await?;

    if row.safety_score.is_some() {
        let _proof_event = record_system_event(
            &state,
            SystemProofEventInput {
                user_id: user.id,
                event_type: "proof_of_safety_score_recorded".to_string(),
                subject_type: "lab_safety".to_string(),
                subject_id: row.lab_id.clone(),
                level: Some(row.level.clone()),
                track: Some("lab".to_string()),
                source_table: "lab_attempts".to_string(),
                source_id: row.id,
                payload: json!({
                    "lab_id": row.lab_id.clone(),
                    "level": row.level.clone(),
                    "safety_score": row.safety_score,
                    "source": "lab.lab_attempts.safety"
                }),
            },
        )
        .await?;
    }

    Ok((StatusCode::CREATED, Json(LabAttemptResponse::from(row))))
}

async fn record_lab_checkpoint_result(
    Path(checkpoint_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LabCheckpointResultRequest>,
) -> Result<(StatusCode, Json<CheckpointResultResponse>), ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let checkpoint_id = clean_required_id(&checkpoint_id, "checkpoint_id")?;
    let lab_id = clean_required_id(&payload.lab_id, "lab_id")?;
    let level = normalize_level(&payload.level)?;
    let score = validate_score(payload.score, "score")?;
    let passed = passed_from_score(score, payload.passed);
    let payload = payload_or_empty(payload.payload);

    let row = sqlx::query_as::<_, CheckpointResultRow>(
        r#"
        insert into checkpoint_results (
            user_id, track, subject_id, checkpoint_id, level, score, passed, payload
        )
        values ($1, 'lab', $2, $3, $4, $5, $6, $7)
        returning id, track, subject_id, checkpoint_id, level, score, passed, created_at
        "#,
    )
    .bind(user.id)
    .bind(lab_id)
    .bind(checkpoint_id)
    .bind(level)
    .bind(score)
    .bind(passed)
    .bind(payload)
    .fetch_one(&state.db)
    .await?;

    let _proof_event = record_system_event(
        &state,
        SystemProofEventInput {
            user_id: user.id,
            event_type: "lab_checkpoint_result_recorded".to_string(),
            subject_type: "lab_checkpoint".to_string(),
            subject_id: row.checkpoint_id.clone(),
            level: Some(row.level.clone()),
            track: Some(row.track.clone()),
            source_table: "checkpoint_results".to_string(),
            source_id: row.id,
            payload: json!({
                "track": row.track.clone(),
                "subject_id": row.subject_id.clone(),
                "checkpoint_id": row.checkpoint_id.clone(),
                "level": row.level.clone(),
                "score": row.score,
                "passed": row.passed,
                "source": "lab.checkpoint_results"
            }),
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(CheckpointResultResponse::from(row))))
}

async fn record_lab_exam_attempt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LabExamAttemptRequest>,
) -> Result<(StatusCode, Json<ExamAttemptResponse>), ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let level = normalize_level(&payload.level)?;
    let exam_id = clean_required_id(&payload.exam_id, "exam_id")?;
    let score = validate_score(payload.score, "score")?;
    let passed = passed_from_score(score, payload.passed);
    let exam_version = payload
        .exam_version
        .as_deref()
        .map(|value| clean_required_id(value, "exam_version"))
        .transpose()?
        .unwrap_or_else(|| "v1".to_string());
    let payload = payload_or_empty(payload.payload);

    let attempt_number = sqlx::query_scalar::<_, i32>(
        r#"
        select coalesce(max(attempt_number), 0) + 1
        from exam_attempts
        where user_id = $1 and track = 'lab' and level = $2 and exam_id = $3
        "#,
    )
    .bind(user.id)
    .bind(&level)
    .bind(&exam_id)
    .fetch_one(&state.db)
    .await?;

    let row = sqlx::query_as::<_, ExamAttemptRow>(
        r#"
        insert into exam_attempts (
            user_id, track, level, exam_id, score, passed, attempt_number, exam_version, payload
        )
        values ($1, 'lab', $2, $3, $4, $5, $6, $7, $8)
        returning id, track, level, exam_id, score, passed, attempt_number, exam_version, created_at
        "#,
    )
    .bind(user.id)
    .bind(level)
    .bind(exam_id)
    .bind(score)
    .bind(passed)
    .bind(attempt_number)
    .bind(exam_version)
    .bind(payload)
    .fetch_one(&state.db)
    .await?;

    let _proof_event = record_system_event(
        &state,
        SystemProofEventInput {
            user_id: user.id,
            event_type: "lab_level_exam_attempt_recorded".to_string(),
            subject_type: "lab_level_exam".to_string(),
            subject_id: row.exam_id.clone(),
            level: Some(row.level.clone()),
            track: Some(row.track.clone()),
            source_table: "exam_attempts".to_string(),
            source_id: row.id,
            payload: json!({
                "track": row.track.clone(),
                "level": row.level.clone(),
                "exam_id": row.exam_id.clone(),
                "score": row.score,
                "passed": row.passed,
                "attempt_number": row.attempt_number,
                "exam_version": row.exam_version.clone(),
                "source": "lab.exam_attempts"
            }),
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(ExamAttemptResponse::from(row))))
}

impl From<LabAttemptRow> for LabAttemptResponse {
    fn from(row: LabAttemptRow) -> Self {
        Self {
            id: row.id,
            lab_id: row.lab_id,
            level: row.level,
            status: row.status,
            score: row.score,
            safety_score: row.safety_score,
            started_at: row.started_at,
            completed_at: row.completed_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<CheckpointResultRow> for CheckpointResultResponse {
    fn from(row: CheckpointResultRow) -> Self {
        Self {
            id: row.id,
            track: row.track,
            subject_id: row.subject_id,
            checkpoint_id: row.checkpoint_id,
            level: row.level,
            score: row.score,
            passed: row.passed,
            created_at: row.created_at,
        }
    }
}

impl From<ExamAttemptRow> for ExamAttemptResponse {
    fn from(row: ExamAttemptRow) -> Self {
        Self {
            id: row.id,
            track: row.track,
            level: row.level,
            exam_id: row.exam_id,
            score: row.score,
            passed: row.passed,
            attempt_number: row.attempt_number,
            exam_version: row.exam_version,
            created_at: row.created_at,
        }
    }
}
