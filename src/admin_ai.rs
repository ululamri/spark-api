use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{ai_runtime, moderation, state::AppState};

const ADMIN_HEADER: &str = "x-karyra-admin-token";

#[derive(Serialize)]
struct AdminEnvelope<T> {
    ok: bool,
    data: T,
    generated_at: DateTime<Utc>,
}

fn success<T>(data: T) -> Json<AdminEnvelope<T>> {
    Json(AdminEnvelope { ok: true, data, generated_at: Utc::now() })
}

#[derive(Debug)]
enum AdminAiError {
    NotConfigured,
    Unauthorized,
    BadRequest(String),
    Database(sqlx::Error),
    Service(String),
}

#[derive(Serialize)]
struct AdminErrorEnvelope {
    ok: bool,
    error: AdminErrorBody,
}

#[derive(Serialize)]
struct AdminErrorBody {
    code: &'static str,
    message: String,
}

impl IntoResponse for AdminAiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::NotConfigured => (StatusCode::SERVICE_UNAVAILABLE, "admin_not_configured", "Admin access is not configured.".to_string()),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "admin_unauthorized", "Admin access is not authorized.".to_string()),
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "admin_bad_request", message),
            Self::Service(message) => (StatusCode::SERVICE_UNAVAILABLE, "admin_ai_unavailable", message),
            Self::Database(error) => {
                tracing::error!(?error, "admin AI database operation failed");
                (StatusCode::INTERNAL_SERVER_ERROR, "admin_internal_error", "The admin AI request could not be completed.".to_string())
            }
        };
        (status, Json(AdminErrorEnvelope { ok: false, error: AdminErrorBody { code, message } })).into_response()
    }
}

impl From<sqlx::Error> for AdminAiError {
    fn from(value: sqlx::Error) -> Self { Self::Database(value) }
}

impl From<crate::error::ApiError> for AdminAiError {
    fn from(value: crate::error::ApiError) -> Self {
        match value {
            crate::error::ApiError::BadRequest(message) => Self::BadRequest(message),
            crate::error::ApiError::ServiceUnavailable(message) => Self::Service(message),
            crate::error::ApiError::RateLimited(message) => Self::Service(message),
            crate::error::ApiError::Unauthorized => Self::Unauthorized,
            crate::error::ApiError::Conflict(message) => Self::BadRequest(message),
            crate::error::ApiError::Internal => Self::Service("AI runtime failed".to_string()),
        }
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/scope", get(scope))
        .route("/settings", get(settings).post(update_settings))
        .route("/moderate-text", post(moderate_text))
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<(), AdminAiError> {
    let configured = state.config.admin_token.as_deref().ok_or(AdminAiError::NotConfigured)?;
    let supplied = headers
        .get(ADMIN_HEADER)
        .and_then(|value| value.to_str().ok())
        .ok_or(AdminAiError::Unauthorized)?;

    if Sha256::digest(configured.as_bytes()) == Sha256::digest(supplied.as_bytes()) {
        Ok(())
    } else {
        Err(AdminAiError::Unauthorized)
    }
}

#[derive(Serialize)]
struct ScopeData {
    module: &'static str,
    phase: &'static str,
    routes: Vec<&'static str>,
    provider_mode: &'static str,
    external_api_policy: &'static str,
    auto_action: bool,
}

async fn scope(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<AdminEnvelope<ScopeData>>, AdminAiError> {
    authorize(&state, &headers)?;
    Ok(success(ScopeData {
        module: module_path!(),
        phase: "ai-moderation-foundation",
        routes: vec![
            "GET /api/admin/ai/scope",
            "GET /api/admin/ai/settings",
            "POST /api/admin/ai/settings",
            "POST /api/admin/ai/moderate-text",
        ],
        provider_mode: "local_first_optional_external_fallback",
        external_api_policy: "admin_only_and_disabled_by_default",
        auto_action: false,
    }))
}

#[derive(Serialize, FromRow)]
struct ProviderSettingItem {
    provider: String,
    enabled: bool,
    mode: String,
    base_url: Option<String>,
    model: Option<String>,
    priority: i32,
    timeout_ms: i32,
    metadata: Value,
    updated_at: DateTime<Utc>,
}

#[derive(Serialize, FromRow)]
struct AssistantSettingItem {
    key: String,
    enabled: bool,
    value: Value,
    locked: bool,
    description: String,
    updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct SettingsData {
    providers: Vec<ProviderSettingItem>,
    assistants: Vec<AssistantSettingItem>,
    openai_key_configured: bool,
    local_base_url_env: String,
    user_model_env: String,
    guard_model_env: String,
}

async fn settings(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<AdminEnvelope<SettingsData>>, AdminAiError> {
    authorize(&state, &headers)?;
    Ok(success(fetch_settings(&state).await?))
}

#[derive(Debug, Deserialize)]
struct UpdateSettingsRequest {
    providers: Option<Vec<ProviderPatch>>,
    assistants: Option<Vec<AssistantPatch>>,
}

#[derive(Debug, Deserialize)]
struct ProviderPatch {
    provider: String,
    enabled: Option<bool>,
    mode: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
    timeout_ms: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct AssistantPatch {
    key: String,
    enabled: Option<bool>,
    value: Option<Value>,
}

async fn update_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateSettingsRequest>,
) -> Result<Json<AdminEnvelope<SettingsData>>, AdminAiError> {
    authorize(&state, &headers)?;

    if let Some(providers) = payload.providers {
        for patch in providers {
            update_provider(&state, patch).await?;
        }
    }
    if let Some(assistants) = payload.assistants {
        for patch in assistants {
            update_assistant(&state, patch).await?;
        }
    }

    Ok(success(fetch_settings(&state).await?))
}

async fn fetch_settings(state: &AppState) -> Result<SettingsData, AdminAiError> {
    let providers = sqlx::query_as::<_, ProviderSettingItem>(
        r#"
        select provider, enabled, mode, base_url, model, priority, timeout_ms, metadata, updated_at
        from ai_provider_settings
        order by priority asc, provider asc
        "#,
    )
    .fetch_all(&state.db)
    .await?;
    let assistants = sqlx::query_as::<_, AssistantSettingItem>(
        r#"
        select key, enabled, value, locked, description, updated_at
        from ai_assistant_settings
        order by key asc
        "#,
    )
    .fetch_all(&state.db)
    .await?;
    Ok(SettingsData {
        providers,
        assistants,
        openai_key_configured: state.config.openai_api_key.is_some(),
        local_base_url_env: state.config.ai_local_base_url.clone(),
        user_model_env: state.config.ai_user_model.clone(),
        guard_model_env: state.config.ai_guard_model.clone(),
    })
}

async fn update_provider(state: &AppState, patch: ProviderPatch) -> Result<(), AdminAiError> {
    let provider = normalize_provider(&patch.provider)?;
    let mode = patch.mode.as_deref().map(normalize_mode).transpose()?;
    let timeout_ms = patch.timeout_ms.map(|value| value.clamp(1000, 120000));
    sqlx::query(
        r#"
        update ai_provider_settings set
          enabled = coalesce($2, enabled),
          mode = coalesce($3, mode),
          base_url = coalesce($4, base_url),
          model = coalesce($5, model),
          timeout_ms = coalesce($6, timeout_ms),
          updated_at = now()
        where provider = $1
        "#,
    )
    .bind(provider)
    .bind(patch.enabled)
    .bind(mode)
    .bind(clean_optional(patch.base_url, 512)?)
    .bind(clean_optional(patch.model, 120)?)
    .bind(timeout_ms)
    .execute(&state.db)
    .await?;
    Ok(())
}

async fn update_assistant(state: &AppState, patch: AssistantPatch) -> Result<(), AdminAiError> {
    let key = normalize_assistant_key(&patch.key)?;
    let locked = sqlx::query_scalar::<_, bool>("select locked from ai_assistant_settings where key = $1")
        .bind(&key)
        .fetch_optional(&state.db)
        .await?
        .unwrap_or(false);
    if locked {
        return Err(AdminAiError::BadRequest("locked assistant setting cannot be changed".to_string()));
    }
    sqlx::query(
        r#"
        update ai_assistant_settings set
          enabled = coalesce($2, enabled),
          value = coalesce($3, value),
          updated_at = now()
        where key = $1
        "#,
    )
    .bind(key)
    .bind(patch.enabled)
    .bind(patch.value)
    .execute(&state.db)
    .await?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct ModerateTextRequest {
    text: String,
    target_type: Option<String>,
    target_id: Option<Uuid>,
    use_external_fallback: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ModerateTextResponse {
    moderation_event_id: Uuid,
    rule_decision: ModerationDecisionData,
    local_ai: Option<AiRunData>,
    external_ai: Option<AiRunData>,
    final_decision: String,
    final_categories: Vec<String>,
    final_score: f32,
    recommendation: String,
}

#[derive(Debug, Serialize)]
struct ModerationDecisionData {
    decision: String,
    categories: Vec<String>,
    score: f32,
    summary: String,
}

#[derive(Debug, Serialize)]
struct AiRunData {
    provider: String,
    model: String,
    decision: String,
    categories: Vec<String>,
    score: f32,
    summary: String,
    latency_ms: i64,
}

async fn moderate_text(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ModerateTextRequest>,
) -> Result<Json<AdminEnvelope<ModerateTextResponse>>, AdminAiError> {
    authorize(&state, &headers)?;
    let text = clean_text(&payload.text)?;
    let target_type = payload.target_type.as_deref().unwrap_or("system");
    let target_type = normalize_target_type(target_type)?;
    let target_id = payload.target_id;
    let use_external = payload.use_external_fallback.unwrap_or(true);

    let rule = moderation::evaluate_text(&text);
    let moderation_event_id = Uuid::new_v4();
    let score = rule.score.unwrap_or(0.0).clamp(0.0, 1.0);
    sqlx::query(
        r#"
        insert into moderation_events (
          id, actor_user_id, target_type, target_id, target_owner_user_id,
          event_type, decision, categories, severity, score, source,
          user_message, admin_summary, metadata
        ) values ($1, null, $2, $3, null, 'ai_review', $4, $5, $6, $7::numeric, 'combined', $8, $9, $10)
        "#,
    )
    .bind(moderation_event_id)
    .bind(target_type)
    .bind(target_id)
    .bind(rule.decision)
    .bind(rule.categories.clone())
    .bind(rule.severity)
    .bind(format!("{score:.5}"))
    .bind(&rule.user_message)
    .bind("Admin AI review initialized from rule-based policy signal.")
    .bind(json!({"manual_review": true, "text_len": text.chars().count()}))
    .execute(&state.db)
    .await?;

    let local_ai = match ai_runtime::provider(&state, "ollama_local").await? {
        Some(provider) if provider.enabled => {
            let output = ai_runtime::ollama_moderate_text(&state, &provider, &text).await?;
            ai_runtime::log_model_run(&state, Some(moderation_event_id), &output, "text").await?;
            Some(AiRunData::from(output))
        }
        _ => None,
    };

    let should_external = use_external && should_use_external(&rule, local_ai.as_ref());
    let external_ai = if should_external {
        match ai_runtime::provider(&state, "openai_moderation").await? {
            Some(provider) if provider.enabled => {
                let output = ai_runtime::openai_moderate_text(&state, &provider, &text).await?;
                ai_runtime::log_model_run(&state, Some(moderation_event_id), &output, "text").await?;
                Some(AiRunData::from(output))
            }
            _ => None,
        }
    } else {
        None
    };

    let final_decision = choose_decision(&rule, local_ai.as_ref(), external_ai.as_ref());
    let final_categories = merge_categories(&rule.categories, local_ai.as_ref(), external_ai.as_ref());
    let final_score = [
        Some(score),
        local_ai.as_ref().map(|item| item.score),
        external_ai.as_ref().map(|item| item.score),
    ]
    .into_iter()
    .flatten()
    .fold(0.0_f32, f32::max)
    .clamp(0.0, 1.0);
    let recommendation = match final_decision.as_str() {
        "block" => "Recommend block or removal. Admin confirmation is still required.",
        "review" => "Recommend human review before public visibility.",
        _ => "No AI safety objection found. Admin may still review context.",
    }
    .to_string();

    Ok(success(ModerateTextResponse {
        moderation_event_id,
        rule_decision: ModerationDecisionData {
            decision: rule.decision.to_string(),
            categories: rule.categories,
            score,
            summary: rule.admin_summary,
        },
        local_ai,
        external_ai,
        final_decision,
        final_categories,
        final_score,
        recommendation,
    }))
}

impl From<ai_runtime::AiRunOutput> for AiRunData {
    fn from(output: ai_runtime::AiRunOutput) -> Self {
        Self {
            provider: output.provider,
            model: output.model,
            decision: output.decision,
            categories: output.categories,
            score: output.score,
            summary: output.summary,
            latency_ms: output.latency_ms,
        }
    }
}

fn should_use_external(rule: &moderation::ModerationOutcome, local_ai: Option<&AiRunData>) -> bool {
    if rule.is_block() || rule.is_review() {
        return true;
    }
    match local_ai {
        Some(run) => run.decision != "allow" || run.score >= 0.55,
        None => false,
    }
}

fn choose_decision(rule: &moderation::ModerationOutcome, local_ai: Option<&AiRunData>, external_ai: Option<&AiRunData>) -> String {
    let mut decision = rule.decision.to_string();
    for item in [local_ai, external_ai].into_iter().flatten() {
        decision = max_decision(&decision, &item.decision).to_string();
    }
    decision
}

fn max_decision<'a>(current: &'a str, next: &'a str) -> &'a str {
    if decision_rank(next) > decision_rank(current) { next } else { current }
}

fn decision_rank(value: &str) -> i32 {
    match value { "allow" => 0, "review" => 1, "restrict" => 2, "block" => 3, _ => 1 }
}

fn merge_categories(rule_categories: &[String], local_ai: Option<&AiRunData>, external_ai: Option<&AiRunData>) -> Vec<String> {
    let mut output = Vec::<String>::new();
    for category in rule_categories {
        push_unique(&mut output, category);
    }
    for run in [local_ai, external_ai].into_iter().flatten() {
        for category in &run.categories {
            push_unique(&mut output, category);
        }
    }
    output
}

fn push_unique(output: &mut Vec<String>, value: &str) {
    if !output.iter().any(|item| item == value) {
        output.push(value.to_string());
    }
}

fn clean_text(input: &str) -> Result<String, AdminAiError> {
    let value = input.trim();
    if value.is_empty() {
        return Err(AdminAiError::BadRequest("text is required".to_string()));
    }
    if value.chars().count() > 8000 {
        return Err(AdminAiError::BadRequest("text is too long".to_string()));
    }
    if value.chars().any(char::is_control) {
        return Err(AdminAiError::BadRequest("text cannot contain control characters".to_string()));
    }
    Ok(value.to_string())
}

fn clean_optional(input: Option<String>, max: usize) -> Result<Option<String>, AdminAiError> {
    let Some(value) = input else { return Ok(None); };
    let value = value.trim().to_string();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > max {
        return Err(AdminAiError::BadRequest("setting value is too long".to_string()));
    }
    if value.chars().any(char::is_control) {
        return Err(AdminAiError::BadRequest("setting value cannot contain control characters".to_string()));
    }
    Ok(Some(value))
}

fn normalize_provider(input: &str) -> Result<String, AdminAiError> {
    match input.trim() {
        "ollama_local" | "openai_moderation" | "hive_moderation" => Ok(input.trim().to_string()),
        _ => Err(AdminAiError::BadRequest("provider is not supported".to_string())),
    }
}

fn normalize_mode(input: &str) -> Result<String, AdminAiError> {
    match input.trim() {
        "user_assistant" | "admin_only" | "moderation_only" | "disabled" => Ok(input.trim().to_string()),
        _ => Err(AdminAiError::BadRequest("provider mode is not supported".to_string())),
    }
}

fn normalize_assistant_key(input: &str) -> Result<String, AdminAiError> {
    match input.trim() {
        "user_assistant" | "admin_moderation_assistant" => Ok(input.trim().to_string()),
        _ => Err(AdminAiError::BadRequest("assistant setting is not supported".to_string())),
    }
}

fn normalize_target_type(input: &str) -> Result<&'static str, AdminAiError> {
    match input.trim() {
        "post" => Ok("post"),
        "comment" => Ok("comment"),
        "profile" => Ok("profile"),
        "media" => Ok("media"),
        "avatar" => Ok("avatar"),
        "assistant_message" => Ok("assistant_message"),
        "system" => Ok("system"),
        _ => Err(AdminAiError::BadRequest("target_type is not supported".to_string())),
    }
}
