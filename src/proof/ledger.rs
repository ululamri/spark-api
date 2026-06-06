use chrono::{DateTime, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

pub const PROOF_EVENT_SCHEMA_VERSION: &str = "spark-proof-event-v1";
const PROOF_EVENT_ISSUER: &str = "karyra-spark-api";
const GENESIS_HASH: &str = "GENESIS";

#[derive(Debug, Clone)]
pub struct SystemProofEventInput {
    pub user_id: Uuid,
    pub event_type: String,
    pub subject_type: String,
    pub subject_id: String,
    pub level: Option<String>,
    pub track: Option<String>,
    pub source_table: String,
    pub source_id: Uuid,
    pub payload: Value,
}

#[derive(Debug, Clone, FromRow)]
pub struct RecordedProofEvent {
    pub id: Uuid,
    pub event_hash: Option<String>,
    pub evidence_root: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub async fn record_system_event(
    state: &AppState,
    input: SystemProofEventInput,
) -> Result<RecordedProofEvent, ApiError> {
    validate_event_input(&input)?;

    if let Some(existing) = existing_event(state, &input).await? {
        return Ok(existing);
    }

    let previous_event_hash = latest_event_hash(state, input.user_id).await?;
    let event_hash = calculate_event_hash(&input, previous_event_hash.as_deref())?;
    let evidence_root = event_hash.clone();

    let row = sqlx::query_as::<_, RecordedProofEvent>(
        r#"
        insert into proof_events (
            user_id,
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
            payload
        )
        values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        returning id, event_hash, evidence_root, created_at
        "#,
    )
    .bind(input.user_id)
    .bind(input.event_type)
    .bind(input.subject_type)
    .bind(input.subject_id)
    .bind(input.level)
    .bind(input.track)
    .bind(input.source_table)
    .bind(input.source_id)
    .bind(event_hash)
    .bind(previous_event_hash)
    .bind(evidence_root)
    .bind(PROOF_EVENT_ISSUER)
    .bind(PROOF_EVENT_SCHEMA_VERSION)
    .bind(input.payload)
    .fetch_one(&state.db)
    .await?;

    Ok(row)
}

async fn existing_event(
    state: &AppState,
    input: &SystemProofEventInput,
) -> Result<Option<RecordedProofEvent>, ApiError> {
    let row = sqlx::query_as::<_, RecordedProofEvent>(
        r#"
        select id, event_hash, evidence_root, created_at
        from proof_events
        where user_id = $1
          and source_table = $2
          and source_id = $3
          and event_type = $4
        limit 1
        "#,
    )
    .bind(input.user_id)
    .bind(&input.source_table)
    .bind(input.source_id)
    .bind(&input.event_type)
    .fetch_optional(&state.db)
    .await?;

    Ok(row)
}

async fn latest_event_hash(state: &AppState, user_id: Uuid) -> Result<Option<String>, ApiError> {
    let hash = sqlx::query_scalar::<_, String>(
        r#"
        select event_hash
        from proof_events
        where user_id = $1
          and event_hash is not null
        order by created_at desc, id desc
        limit 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?;

    Ok(hash)
}

fn calculate_event_hash(
    input: &SystemProofEventInput,
    previous_event_hash: Option<&str>,
) -> Result<String, ApiError> {
    let payload = serde_json::to_string(&input.payload).map_err(|error| {
        tracing::error!(?error, "failed to serialize proof event payload");
        ApiError::Internal
    })?;

    let material = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        PROOF_EVENT_SCHEMA_VERSION,
        input.user_id,
        input.event_type,
        input.subject_type,
        input.subject_id,
        input.level.as_deref().unwrap_or(""),
        input.track.as_deref().unwrap_or(""),
        input.source_table,
        input.source_id,
        previous_event_hash.unwrap_or(GENESIS_HASH),
        payload
    );

    Ok(sha256_hex(material.as_bytes()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();

    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn validate_event_input(input: &SystemProofEventInput) -> Result<(), ApiError> {
    validate_segment(&input.event_type, "event_type")?;
    validate_segment(&input.subject_type, "subject_type")?;
    validate_segment(&input.subject_id, "subject_id")?;
    validate_segment(&input.source_table, "source_table")?;

    if let Some(level) = input.level.as_deref() {
        validate_segment(level, "level")?;
    }

    if let Some(track) = input.track.as_deref() {
        validate_segment(track, "track")?;
    }

    Ok(())
}

fn validate_segment(value: &str, field: &str) -> Result<(), ApiError> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Err(ApiError::BadRequest(format!("{field} is required")));
    }

    if trimmed.chars().count() > 128 {
        return Err(ApiError::BadRequest(format!("{field} is too long")));
    }

    if trimmed.chars().any(char::is_control) {
        return Err(ApiError::BadRequest(format!(
            "{field} cannot contain control characters"
        )));
    }

    Ok(())
}
