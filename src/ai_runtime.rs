use std::time::Duration;

use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AiProviderRow {
    pub provider: String,
    pub enabled: bool,
    pub mode: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub priority: i32,
    pub timeout_ms: i32,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiRunOutput {
    pub provider: String,
    pub model: String,
    pub decision: String,
    pub categories: Vec<String>,
    pub score: f32,
    pub summary: String,
    pub raw_response: Value,
    pub latency_ms: i64,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: Option<OllamaMessage>,
    response: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaMessage {
    content: String,
}

pub async fn provider(state: &AppState, provider: &str) -> Result<Option<AiProviderRow>, ApiError> {
    let row = sqlx::query_as::<_, AiProviderRow>(
        r#"
        select provider, enabled, mode, base_url, model, priority, timeout_ms, metadata
        from ai_provider_settings
        where provider = $1
        "#,
    )
    .bind(provider)
    .fetch_optional(&state.db)
    .await?;
    Ok(row)
}

pub async fn ollama_chat(
    state: &AppState,
    provider: &AiProviderRow,
    model_fallback: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<AiRunOutput, ApiError> {
    if !provider.enabled {
        return Err(ApiError::ServiceUnavailable("local AI provider is disabled".to_string()));
    }

    let base_url = provider
        .base_url
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(&state.config.ai_local_base_url)
        .trim_end_matches('/')
        .to_string();
    let model = provider
        .model
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(model_fallback)
        .to_string();
    let timeout = provider.timeout_ms.clamp(1000, 120000) as u64;
    let started = Utc::now();

    let client = Client::builder()
        .timeout(Duration::from_millis(timeout))
        .build()
        .map_err(|_| ApiError::ServiceUnavailable("could not create AI HTTP client".to_string()))?;

    let response = client
        .post(format!("{base_url}/api/chat"))
        .json(&json!({
            "model": &model,
            "stream": false,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ]
        }))
        .send()
        .await
        .map_err(|error| ApiError::ServiceUnavailable(format!("local AI request failed: {error}")))?;

    if !response.status().is_success() {
        return Err(ApiError::ServiceUnavailable(format!(
            "local AI provider returned HTTP {}",
            response.status()
        )));
    }

    let raw: Value = response
        .json()
        .await
        .map_err(|_| ApiError::ServiceUnavailable("local AI response was not valid JSON".to_string()))?;
    let parsed: OllamaChatResponse = serde_json::from_value(raw.clone())
        .map_err(|_| ApiError::ServiceUnavailable("local AI response shape was not recognized".to_string()))?;
    let content = parsed
        .message
        .map(|message| message.content)
        .or(parsed.response)
        .unwrap_or_default();
    let latency_ms = Utc::now()
        .signed_duration_since(started)
        .num_milliseconds()
        .max(0);

    Ok(AiRunOutput {
        provider: provider.provider.clone(),
        model,
        decision: "assist".to_string(),
        categories: Vec::new(),
        score: 0.0,
        summary: content,
        raw_response: raw,
        latency_ms,
    })
}

pub async fn ollama_moderate_text(
    state: &AppState,
    provider: &AiProviderRow,
    text: &str,
) -> Result<AiRunOutput, ApiError> {
    let system_prompt = r#"You are Karyra Spark's local safety classifier. Return a compact JSON object only with keys: decision, categories, score, summary. Decisions are allow, review, or block. Categories may include gambling, trading_solicitation, financial_scam, phishing, private_key_request, malware, harassment, hate, violence_threat, doxxing, advertising, external_link_risk, toxicity_low, unsafe_media. Preserve educational blockchain and wallet-safety discussion when it is clearly defensive or learning-oriented."#;
    let user_prompt = format!("Classify this community content for safety review:\n\n{text}");
    let mut output = ollama_chat(
        state,
        provider,
        &state.config.ai_guard_model,
        system_prompt,
        &user_prompt,
    )
    .await?;

    if let Some(parsed) = parse_classifier_json(&output.summary) {
        output.decision = parsed.decision;
        output.categories = parsed.categories;
        output.score = parsed.score;
        output.summary = parsed.summary;
    } else {
        output.decision = "review".to_string();
        output.score = 0.55;
        output.summary = "Local AI returned non-structured output; route to review.".to_string();
    }
    Ok(output)
}

pub async fn openai_moderate_text(
    state: &AppState,
    provider: &AiProviderRow,
    text: &str,
) -> Result<AiRunOutput, ApiError> {
    if !provider.enabled {
        return Err(ApiError::ServiceUnavailable("external moderation provider is disabled".to_string()));
    }
    let api_key = state
        .config
        .openai_api_key
        .as_deref()
        .ok_or_else(|| ApiError::ServiceUnavailable("OPENAI_API_KEY is not configured".to_string()))?;
    let model = provider
        .model
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("omni-moderation-latest")
        .to_string();
    let timeout = provider.timeout_ms.clamp(1000, 120000) as u64;
    let started = Utc::now();

    let client = Client::builder()
        .timeout(Duration::from_millis(timeout))
        .build()
        .map_err(|_| ApiError::ServiceUnavailable("could not create AI HTTP client".to_string()))?;

    let response = client
        .post("https://api.openai.com/v1/moderations")
        .bearer_auth(api_key)
        .json(&json!({"model": &model, "input": text}))
        .send()
        .await
        .map_err(|error| ApiError::ServiceUnavailable(format!("external moderation request failed: {error}")))?;

    if !response.status().is_success() {
        return Err(ApiError::ServiceUnavailable(format!(
            "external moderation provider returned HTTP {}",
            response.status()
        )));
    }

    let raw: Value = response
        .json()
        .await
        .map_err(|_| ApiError::ServiceUnavailable("external moderation response was not valid JSON".to_string()))?;
    let latency_ms = Utc::now()
        .signed_duration_since(started)
        .num_milliseconds()
        .max(0);
    let result = raw
        .get("results")
        .and_then(|value| value.as_array())
        .and_then(|items| items.first())
        .cloned()
        .unwrap_or_else(|| json!({}));
    let flagged = result.get("flagged").and_then(|value| value.as_bool()).unwrap_or(false);
    let categories = result
        .get("categories")
        .and_then(|value| value.as_object())
        .map(|object| {
            object
                .iter()
                .filter_map(|(key, value)| value.as_bool().filter(|flag| *flag).map(|_| key.clone()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let score = result
        .get("category_scores")
        .and_then(|value| value.as_object())
        .and_then(|object| object.values().filter_map(|value| value.as_f64()).max_by(|a, b| a.total_cmp(b)))
        .unwrap_or(if flagged { 0.85 } else { 0.0 }) as f32;

    Ok(AiRunOutput {
        provider: provider.provider.clone(),
        model,
        decision: if flagged { "review".to_string() } else { "allow".to_string() },
        categories,
        score,
        summary: if flagged {
            "External moderation provider flagged this content for admin review.".to_string()
        } else {
            "External moderation provider did not flag this content.".to_string()
        },
        raw_response: raw,
        latency_ms,
    })
}

pub async fn log_model_run(state: &AppState, moderation_event_id: Option<Uuid>, output: &AiRunOutput, input_type: &str) -> Result<Uuid, ApiError> {
    let id = Uuid::new_v4();
    let score = format!("{:.5}", output.score.clamp(0.0, 1.0));
    sqlx::query(
        r#"
        insert into moderation_model_runs (
          id, moderation_event_id, provider, model, input_type, decision,
          categories, score, latency_ms, raw_response
        ) values ($1, $2, $3, $4, $5, $6, $7, $8::numeric, $9, $10)
        "#,
    )
    .bind(id)
    .bind(moderation_event_id)
    .bind(&output.provider)
    .bind(&output.model)
    .bind(input_type)
    .bind(normalize_decision_for_model_run(&output.decision))
    .bind(output.categories.clone())
    .bind(score)
    .bind(output.latency_ms as i32)
    .bind(&output.raw_response)
    .execute(&state.db)
    .await?;
    Ok(id)
}

#[derive(Debug, Deserialize)]
struct ClassifierJson {
    decision: String,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    score: f32,
    #[serde(default)]
    summary: String,
}

fn parse_classifier_json(content: &str) -> Option<ClassifierJson> {
    let trimmed = content.trim();
    if let Ok(parsed) = serde_json::from_str::<ClassifierJson>(trimmed) {
        return Some(normalize_classifier(parsed));
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end <= start {
        return None;
    }
    serde_json::from_str::<ClassifierJson>(&trimmed[start..=end])
        .ok()
        .map(normalize_classifier)
}

fn normalize_classifier(mut parsed: ClassifierJson) -> ClassifierJson {
    parsed.decision = match parsed.decision.trim().to_ascii_lowercase().as_str() {
        "allow" => "allow".to_string(),
        "block" => "block".to_string(),
        "restrict" => "restrict".to_string(),
        _ => "review".to_string(),
    };
    parsed.score = parsed.score.clamp(0.0, 1.0);
    parsed
}

fn normalize_decision_for_model_run(decision: &str) -> &'static str {
    match decision {
        "allow" | "assist" => "allow",
        "block" => "block",
        "restrict" => "restrict",
        _ => "review",
    }
}
