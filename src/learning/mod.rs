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
        clean_required_id, normalize_level, normalize_progress_status, passed_from_score,
        payload_or_empty, progress_percent, validate_score,
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
pub struct LessonProgressRequest {
    pub level: String,
    pub status: Option<String>,
    pub progress_percent: Option<i32>,
    pub completed: Option<bool>,
    pub payload: Option<Value>,
}

#[derive(Debug, Serialize)]
struct LessonProgressResponse {
    id: Uuid,
    lesson_id: String,
    level: String,
    status: String,
    progress_percent: i32,
    completed_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct LessonProgressRow {
    id: Uuid,
    lesson_id: String,
    level: String,
    status: String,
    progress_percent: i32,
    completed_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CheckpointResultRequest {
    pub lesson_id: String,
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
pub struct ExamAttemptRequest {
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
        .route("/me/progress", get(list_my_lesson_progress))
        .route("/lessons/{lesson_id}/progress", post(upsert_lesson_progress))
        .route(
            "/checkpoints/{checkpoint_id}/results",
            post(record_checkpoint_result),
        )
        .route("/exam-attempts", post(record_core_exam_attempt))
}

async fn scope() -> Json<ScopeResponse> {
    Json(ScopeResponse {
        module: module_path!(),
        phase: "learning-progress-api+proof-ledger",
        implemented_now: vec![
            "authenticated-lesson-progress",
            "checkpoint-result-recording",
            "core-level-exam-attempt-recording",
            "system-proof-source-foundation",
        ],
        next_backend_steps: vec![
            "passport-eligibility-aggregation",
            "eligibility-aggregation",
            "frontend-session-hydration",
            "level-completion-policy",
        ],
    })
}

async fn list_my_lesson_progress(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ListResponse<LessonProgressResponse>>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let rows = sqlx::query_as::<_, LessonProgressRow>(
        r#"
        select id, lesson_id, level, status, progress_percent, completed_at, updated_at
        from lesson_progress
        where user_id = $1
        order by updated_at desc
        limit 100
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(ListResponse {
        items: rows.into_iter().map(LessonProgressResponse::from).collect(),
    }))
}

async fn upsert_lesson_progress(
    Path(lesson_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LessonProgressRequest>,
) -> Result<(StatusCode, Json<LessonProgressResponse>), ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let lesson_id = clean_required_id(&lesson_id, "lesson_id")?;
    let level = normalize_level(&payload.level)?;
    let completed = payload.completed.unwrap_or(false);
    let status = normalize_progress_status(payload.status.as_deref(), completed)?;
    let progress_percent = progress_percent(payload.progress_percent, status == "completed")?;
    let completed_at = if status == "completed" { Some(Utc::now()) } else { None };
    let payload = payload_or_empty(payload.payload);

    let row = sqlx::query_as::<_, LessonProgressRow>(
        r#"
        insert into lesson_progress (
            user_id, lesson_id, level, status, progress_percent, completed_at, payload
        )
        values ($1, $2, $3, $4, $5, $6, $7)
        on conflict (user_id, lesson_id) do update set
          level = excluded.level,
          status = excluded.status,
          progress_percent = greatest(lesson_progress.progress_percent, excluded.progress_percent),
          completed_at = coalesce(excluded.completed_at, lesson_progress.completed_at),
          payload = excluded.payload,
          updated_at = now()
        returning id, lesson_id, level, status, progress_percent, completed_at, updated_at
        "#,
    )
    .bind(user.id)
    .bind(lesson_id)
    .bind(level)
    .bind(status)
    .bind(progress_percent)
    .bind(completed_at)
    .bind(payload)
    .fetch_one(&state.db)
    .await?;

    if row.status == "completed" {
        let _proof_event = record_system_event(
            &state,
            SystemProofEventInput {
                user_id: user.id,
                event_type: "proof_of_learning_lesson_completed".to_string(),
                subject_type: "lesson".to_string(),
                subject_id: row.lesson_id.clone(),
                level: Some(row.level.clone()),
                track: Some("core".to_string()),
                source_table: "lesson_progress".to_string(),
                source_id: row.id,
                payload: json!({
                    "lesson_id": row.lesson_id.clone(),
                    "level": row.level.clone(),
                    "status": row.status.clone(),
                    "progress_percent": row.progress_percent,
                    "completed_at": row.completed_at.clone(),
                    "source": "learning.lesson_progress"
                }),
            },
        )
        .await?;
    }

    Ok((StatusCode::OK, Json(LessonProgressResponse::from(row))))
}

async fn record_checkpoint_result(
    Path(checkpoint_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CheckpointResultRequest>,
) -> Result<(StatusCode, Json<CheckpointResultResponse>), ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let checkpoint_id = clean_required_id(&checkpoint_id, "checkpoint_id")?;
    let lesson_id = clean_required_id(&payload.lesson_id, "lesson_id")?;
    let level = normalize_level(&payload.level)?;
    let score = validate_score(payload.score, "score")?;
    let passed = passed_from_score(score, payload.passed);
    let payload = payload_or_empty(payload.payload);

    let row = sqlx::query_as::<_, CheckpointResultRow>(
        r#"
        insert into checkpoint_results (
            user_id, track, subject_id, checkpoint_id, level, score, passed, payload
        )
        values ($1, 'core', $2, $3, $4, $5, $6, $7)
        returning id, track, subject_id, checkpoint_id, level, score, passed, created_at
        "#,
    )
    .bind(user.id)
    .bind(lesson_id)
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
            event_type: "core_checkpoint_result_recorded".to_string(),
            subject_type: "checkpoint".to_string(),
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
                "source": "learning.checkpoint_results"
            }),
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(CheckpointResultResponse::from(row))))
}

async fn record_core_exam_attempt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ExamAttemptRequest>,
) -> Result<(StatusCode, Json<ExamAttemptResponse>), ApiError> {
    record_exam_attempt(&state, &headers, payload, "core").await
}

async fn record_exam_attempt(
    state: &AppState,
    headers: &HeaderMap,
    payload: ExamAttemptRequest,
    track: &str,
) -> Result<(StatusCode, Json<ExamAttemptResponse>), ApiError> {
    let user = require_current_user(state, headers).await?;
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
        where user_id = $1 and track = $2 and level = $3 and exam_id = $4
        "#,
    )
    .bind(user.id)
    .bind(track)
    .bind(&level)
    .bind(&exam_id)
    .fetch_one(&state.db)
    .await?;

    let row = sqlx::query_as::<_, ExamAttemptRow>(
        r#"
        insert into exam_attempts (
            user_id, track, level, exam_id, score, passed, attempt_number, exam_version, payload
        )
        values ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        returning id, track, level, exam_id, score, passed, attempt_number, exam_version, created_at
        "#,
    )
    .bind(user.id)
    .bind(track)
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
        state,
        SystemProofEventInput {
            user_id: user.id,
            event_type: "core_level_exam_attempt_recorded".to_string(),
            subject_type: "level_exam".to_string(),
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
                "source": "learning.exam_attempts"
            }),
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(ExamAttemptResponse::from(row))))
}

impl From<LessonProgressRow> for LessonProgressResponse {
    fn from(row: LessonProgressRow) -> Self {
        Self {
            id: row.id,
            lesson_id: row.lesson_id,
            level: row.level,
            status: row.status,
            progress_percent: row.progress_percent,
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
