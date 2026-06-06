use serde_json::{json, Value};

use crate::error::ApiError;

pub fn clean_required_id(input: &str, field: &str) -> Result<String, ApiError> {
    let value = input.trim();

    if value.is_empty() {
        return Err(ApiError::BadRequest(format!("{field} is required")));
    }

    if value.chars().count() > 128 {
        return Err(ApiError::BadRequest(format!("{field} is too long")));
    }

    if value.chars().any(char::is_control) {
        return Err(ApiError::BadRequest(format!(
            "{field} cannot contain control characters"
        )));
    }

    Ok(value.to_string())
}

pub fn normalize_level(input: &str) -> Result<String, ApiError> {
    let value = input.trim().to_ascii_lowercase();

    match value.as_str() {
        "beginner" | "intermediate" | "advanced" => Ok(value),
        _ => Err(ApiError::BadRequest(
            "level must be beginner, intermediate, or advanced".to_string(),
        )),
    }
}

pub fn validate_score(score: i32, field: &str) -> Result<i32, ApiError> {
    if (0..=100).contains(&score) {
        Ok(score)
    } else {
        Err(ApiError::BadRequest(format!(
            "{field} must be between 0 and 100"
        )))
    }
}

pub fn validate_optional_score(score: Option<i32>, field: &str) -> Result<Option<i32>, ApiError> {
    score.map(|value| validate_score(value, field)).transpose()
}

pub fn normalize_progress_status(
    input: Option<&str>,
    completed: bool,
) -> Result<String, ApiError> {
    if completed {
        return Ok("completed".to_string());
    }

    let value = input.unwrap_or("in_progress").trim().to_ascii_lowercase();
    match value.as_str() {
        "not_started" | "in_progress" | "completed" => Ok(value),
        _ => Err(ApiError::BadRequest(
            "progress status must be not_started, in_progress, or completed".to_string(),
        )),
    }
}

pub fn normalize_attempt_status(input: Option<&str>) -> Result<String, ApiError> {
    let value = input.unwrap_or("submitted").trim().to_ascii_lowercase();

    match value.as_str() {
        "started" | "submitted" | "passed" | "failed" => Ok(value),
        _ => Err(ApiError::BadRequest(
            "attempt status must be started, submitted, passed, or failed".to_string(),
        )),
    }
}

pub fn progress_percent(input: Option<i32>, completed: bool) -> Result<i32, ApiError> {
    let value = if completed { 100 } else { input.unwrap_or(0) };

    if (0..=100).contains(&value) {
        Ok(value)
    } else {
        Err(ApiError::BadRequest(
            "progress_percent must be between 0 and 100".to_string(),
        ))
    }
}

pub fn payload_or_empty(payload: Option<Value>) -> Value {
    payload.unwrap_or_else(|| json!({}))
}

pub fn passed_from_score(score: i32, passed: Option<bool>) -> bool {
    passed.unwrap_or(score >= 70)
}

pub fn is_final_attempt_status(status: &str) -> bool {
    matches!(status, "submitted" | "passed" | "failed")
}
