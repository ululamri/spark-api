use axum::{
    extract::State,
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    ai_runtime, auth::session::require_current_user, error::ApiError, moderation, state::AppState,
};

#[derive(Serialize)]
struct ScopeResponse {
    module: &'static str,
    phase: &'static str,
    provider_mode: &'static str,
    routes: Vec<&'static str>,
    safeguards: Vec<&'static str>,
}

#[derive(Debug, Deserialize)]
struct UserAssistantRequest {
    message: String,
    context: Option<Value>,
}

#[derive(Debug, Serialize)]
struct UserAssistantResponse {
    reply: String,
    refusal: bool,
    provider: String,
    model: Option<String>,
    generated_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/scope", get(scope))
        .route("/user-assistant/chat", post(user_assistant_chat))
}

async fn scope() -> Json<ScopeResponse> {
    Json(ScopeResponse {
        module: module_path!(),
        phase: "local-learner-assistant-foundation",
        provider_mode: "local_only_by_default",
        routes: vec!["GET /v1/ai/scope", "POST /v1/ai/user-assistant/chat"],
        safeguards: vec![
            "requires authenticated learner session",
            "uses adaptive ai_user_chat rate limit",
            "screens user prompt with content policy before local model call",
            "does not provide trading solicitation or financial advice",
            "does not call external API for learner chat",
        ],
    })
}

async fn user_assistant_chat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UserAssistantRequest>,
) -> Result<Json<UserAssistantResponse>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    moderation::enforce_rate_limit(&state, user.id, "ai_user_chat").await?;

    let message = clean_message(&payload.message)?;
    let safety = moderation::evaluate_text(&message);
    if !safety.is_allow() {
        return Ok(Json(UserAssistantResponse {
            reply: "I can help with learning, wallet safety, and Karyra Spark navigation, but I cannot help with promotional, risky, or unsafe requests.".to_string(),
            refusal: true,
            provider: "policy".to_string(),
            model: None,
            generated_at: Utc::now(),
        }));
    }

    let settings_enabled = sqlx::query_scalar::<_, bool>(
        "select coalesce(enabled, false) from ai_assistant_settings where key = 'user_assistant'",
    )
    .fetch_optional(&state.db)
    .await?
    .unwrap_or(false);

    if !settings_enabled {
        return Err(ApiError::ServiceUnavailable(
            "local learner assistant is disabled".to_string(),
        ));
    }

    let provider = ai_runtime::provider(&state, "ollama_local")
        .await?
        .ok_or_else(|| {
            ApiError::ServiceUnavailable("local AI provider is not configured".to_string())
        })?;

    let context = payload.context.unwrap_or(Value::Null);
    let system_prompt = r#"You are Karyra Spark's local learner assistant. Help users understand lessons, Cairo, Starknet, wallet safety, and platform navigation. Keep responses practical, friendly, and beginner-safe. Do not provide financial advice, trading calls, promotion, private key handling, harmful instructions, or unsafe content. If the user asks for something outside the learning and safety scope, refuse briefly and redirect to learning."#;
    let user_prompt = format!(
        "Learner message:\n{}\n\nOptional platform context:\n{}",
        message, context
    );

    let output = ai_runtime::ollama_chat(
        &state,
        &provider,
        &state.config.ai_user_model,
        system_prompt,
        &user_prompt,
    )
    .await?;
    ai_runtime::log_model_run(&state, None, &output, "text").await?;

    Ok(Json(UserAssistantResponse {
        reply: output.summary,
        refusal: false,
        provider: output.provider,
        model: Some(output.model),
        generated_at: Utc::now(),
    }))
}

fn clean_message(input: &str) -> Result<String, ApiError> {
    let value = input.trim();
    if value.is_empty() {
        return Err(ApiError::BadRequest("message is required".to_string()));
    }
    if value.chars().count() > 4000 {
        return Err(ApiError::BadRequest("message is too long".to_string()));
    }
    if value.chars().any(char::is_control) {
        return Err(ApiError::BadRequest(
            "message cannot contain control characters".to_string(),
        ));
    }
    Ok(value.to_string())
}
