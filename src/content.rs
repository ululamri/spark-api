use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

#[derive(Serialize)]
struct ContentEnvelope<T> {
    ok: bool,
    data: T,
    generated_at: DateTime<Utc>,
}

fn success<T>(data: T) -> Json<ContentEnvelope<T>> {
    Json(ContentEnvelope {
        ok: true,
        data,
        generated_at: Utc::now(),
    })
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/scope", get(scope))
        .route("/lessons", get(lessons))
        .route("/lessons/:slug", get(lesson_detail))
        .route("/labs", get(labs))
        .route("/labs/:slug", get(lab_detail))
}

#[derive(Serialize)]
struct ScopeData {
    module: &'static str,
    phase: &'static str,
    source: &'static str,
    routes: Vec<&'static str>,
    contract: ContentContract,
}

#[derive(Serialize)]
struct ContentContract {
    renderer_contract_version: i32,
    lesson_block_types: Vec<&'static str>,
    lab_step_types: Vec<&'static str>,
    lab_runtime_types: Vec<&'static str>,
}

async fn scope() -> Json<ContentEnvelope<ScopeData>> {
    success(ScopeData {
        module: module_path!(),
        phase: "directus-learn-lab-read-bridge",
        source: "directus-published-content-only",
        routes: vec![
            "GET /v1/content/lessons",
            "GET /v1/content/lessons/:slug",
            "GET /v1/content/labs",
            "GET /v1/content/labs/:slug",
        ],
        contract: ContentContract {
            renderer_contract_version: 1,
            lesson_block_types: LESSON_BLOCK_TYPES.to_vec(),
            lab_step_types: LAB_STEP_TYPES.to_vec(),
            lab_runtime_types: LAB_RUNTIME_TYPES.to_vec(),
        },
    })
}

const LESSON_BLOCK_TYPES: &[&str] = &[
    "story",
    "concept",
    "analogy",
    "media",
    "code",
    "checkpoint",
    "quiz",
    "glossary",
    "reflection",
    "callout",
    "ai_helper_prompt",
];

const LAB_STEP_TYPES: &[&str] = &[
    "instruction",
    "task",
    "shell",
    "code",
    "quiz",
    "checkpoint",
    "hint",
    "expected_output",
    "safety_note",
    "ai_helper_prompt",
];

const LAB_RUNTIME_TYPES: &[&str] = &[
    "browser_only",
    "shell",
    "cairo",
    "scarb",
    "starknet_foundry",
    "dojo",
    "node",
    "rust",
    "plugin",
];

#[derive(Debug, Deserialize)]
struct DirectusList<T> {
    data: Vec<T>,
}

#[derive(Debug, Deserialize)]
struct DirectusItem<T> {
    data: T,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct DirectusLesson {
    id: Uuid,
    course_id: Option<Uuid>,
    slug: String,
    title: String,
    subtitle: String,
    summary: String,
    learning_goal: String,
    estimated_minutes: i32,
    difficulty: String,
    sort_order: i32,
    published_version: Option<i32>,
    published_at: Option<DateTime<Utc>>,
    date_updated: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct DirectusLessonBlock {
    id: Uuid,
    lesson_id: Uuid,
    block_type: String,
    title: String,
    body: String,
    payload: Value,
    sort_order: i32,
    renderer_contract_version: i32,
    date_updated: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct DirectusRuntimeProfile {
    id: Uuid,
    slug: String,
    title: String,
    runtime_type: String,
    sdk_profile: String,
    tool_requirements: Value,
    allowed_commands: Value,
    network_policy: String,
    filesystem_policy: String,
    command_timeout_seconds: i32,
    date_updated: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct DirectusLabModule {
    id: Uuid,
    slug: String,
    title: String,
    summary: String,
    learning_goal: String,
    runtime_profile_id: Option<Uuid>,
    estimated_minutes: i32,
    difficulty: String,
    prerequisite_notes: String,
    published_version: Option<i32>,
    published_at: Option<DateTime<Utc>>,
    date_updated: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct DirectusLabStep {
    id: Uuid,
    lab_module_id: Uuid,
    step_type: String,
    title: String,
    instruction: String,
    starter_files: Value,
    validation_mode: String,
    validation_payload: Value,
    expected_output: String,
    hints: Value,
    safety_notes: String,
    sort_order: i32,
    renderer_contract_version: i32,
    date_updated: DateTime<Utc>,
}

#[derive(Serialize)]
struct LessonListData {
    items: Vec<DirectusLesson>,
    data_source: &'static str,
}

#[derive(Serialize)]
struct LessonDetailData {
    lesson: DirectusLesson,
    blocks: Vec<DirectusLessonBlock>,
    data_source: &'static str,
}

#[derive(Serialize)]
struct LabListData {
    items: Vec<LabListItem>,
    data_source: &'static str,
}

#[derive(Serialize)]
struct LabListItem {
    module: DirectusLabModule,
    runtime_profile: Option<DirectusRuntimeProfile>,
}

#[derive(Serialize)]
struct LabDetailData {
    module: DirectusLabModule,
    runtime_profile: Option<DirectusRuntimeProfile>,
    steps: Vec<DirectusLabStep>,
    data_source: &'static str,
}

async fn lessons(State(state): State<AppState>) -> Result<Json<ContentEnvelope<LessonListData>>, ApiError> {
    ensure_directus_enabled(&state)?;
    let items = directus_list::<DirectusLesson>(
        &state,
        "karyra_lessons",
        &[
            ("filter[status][_eq]", "published"),
            ("sort", "sort_order,slug"),
            ("limit", "100"),
        ],
    )
    .await?;

    Ok(success(LessonListData {
        items,
        data_source: "directus",
    }))
}

async fn lesson_detail(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<ContentEnvelope<LessonDetailData>>, ApiError> {
    ensure_directus_enabled(&state)?;
    let slug = clean_slug(&slug)?;
    let lesson = directus_single::<DirectusLesson>(
        &state,
        "karyra_lessons",
        &[
            ("filter[status][_eq]", "published"),
            ("filter[slug][_eq]", slug.as_str()),
            ("limit", "1"),
        ],
    )
    .await?
    .ok_or_else(|| ApiError::BadRequest("published lesson was not found".to_string()))?;

    let mut blocks = directus_list::<DirectusLessonBlock>(
        &state,
        "karyra_lesson_blocks",
        &[
            ("filter[status][_eq]", "published"),
            ("filter[lesson_id][_eq]", &lesson.id.to_string()),
            ("sort", "sort_order"),
            ("limit", "200"),
        ],
    )
    .await?;
    validate_lesson_blocks(&blocks)?;
    blocks.sort_by_key(|block| block.sort_order);

    Ok(success(LessonDetailData {
        lesson,
        blocks,
        data_source: "directus",
    }))
}

async fn labs(State(state): State<AppState>) -> Result<Json<ContentEnvelope<LabListData>>, ApiError> {
    ensure_directus_enabled(&state)?;
    let modules = directus_list::<DirectusLabModule>(
        &state,
        "karyra_lab_modules",
        &[
            ("filter[status][_eq]", "published"),
            ("sort", "difficulty,slug"),
            ("limit", "100"),
        ],
    )
    .await?;

    let mut items = Vec::with_capacity(modules.len());
    for module in modules {
        let runtime_profile = match module.runtime_profile_id {
            Some(runtime_id) => fetch_runtime_profile(&state, runtime_id).await?,
            None => None,
        };
        if let Some(profile) = runtime_profile.as_ref() {
            validate_runtime_profile(profile)?;
        }
        items.push(LabListItem {
            module,
            runtime_profile,
        });
    }

    Ok(success(LabListData {
        items,
        data_source: "directus",
    }))
}

async fn lab_detail(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<ContentEnvelope<LabDetailData>>, ApiError> {
    ensure_directus_enabled(&state)?;
    let slug = clean_slug(&slug)?;
    let module = directus_single::<DirectusLabModule>(
        &state,
        "karyra_lab_modules",
        &[
            ("filter[status][_eq]", "published"),
            ("filter[slug][_eq]", slug.as_str()),
            ("limit", "1"),
        ],
    )
    .await?
    .ok_or_else(|| ApiError::BadRequest("published lab module was not found".to_string()))?;

    let runtime_profile = match module.runtime_profile_id {
        Some(runtime_id) => fetch_runtime_profile(&state, runtime_id).await?,
        None => None,
    };
    if let Some(profile) = runtime_profile.as_ref() {
        validate_runtime_profile(profile)?;
    }

    let mut steps = directus_list::<DirectusLabStep>(
        &state,
        "karyra_lab_steps",
        &[
            ("filter[status][_eq]", "published"),
            ("filter[lab_module_id][_eq]", &module.id.to_string()),
            ("sort", "sort_order"),
            ("limit", "200"),
        ],
    )
    .await?;
    validate_lab_steps(&steps)?;
    steps.sort_by_key(|step| step.sort_order);

    Ok(success(LabDetailData {
        module,
        runtime_profile,
        steps,
        data_source: "directus",
    }))
}

async fn fetch_runtime_profile(
    state: &AppState,
    runtime_id: Uuid,
) -> Result<Option<DirectusRuntimeProfile>, ApiError> {
    directus_single::<DirectusRuntimeProfile>(
        state,
        "karyra_lab_runtime_profiles",
        &[
            ("filter[status][_eq]", "published"),
            ("filter[id][_eq]", &runtime_id.to_string()),
            ("limit", "1"),
        ],
    )
    .await
}

fn ensure_directus_enabled(state: &AppState) -> Result<(), ApiError> {
    if !state.config.directus_enabled {
        return Err(ApiError::ServiceUnavailable(
            "Directus content bridge is disabled".to_string(),
        ));
    }
    if state.config.directus_base_url.trim().is_empty() {
        return Err(ApiError::ServiceUnavailable(
            "Directus base URL is not configured".to_string(),
        ));
    }
    Ok(())
}

async fn directus_list<T: DeserializeOwned>(
    state: &AppState,
    collection: &str,
    params: &[(&str, &str)],
) -> Result<Vec<T>, ApiError> {
    let url = directus_url(state, collection)?;
    let client = reqwest::Client::new();
    let mut request = client.get(url).query(params);
    if let Some(token) = state.config.directus_static_token.as_deref() {
        request = request.bearer_auth(token);
    }
    let response = request.send().await.map_err(directus_unavailable)?;
    if !response.status().is_success() {
        return Err(ApiError::ServiceUnavailable(format!(
            "Directus returned {} for {}",
            response.status(),
            collection
        )));
    }
    let body = response
        .json::<DirectusList<T>>()
        .await
        .map_err(directus_unavailable)?;
    Ok(body.data)
}

async fn directus_single<T: DeserializeOwned>(
    state: &AppState,
    collection: &str,
    params: &[(&str, &str)],
) -> Result<Option<T>, ApiError> {
    let mut rows = directus_list::<T>(state, collection, params).await?;
    Ok(rows.pop())
}

fn directus_url(state: &AppState, collection: &str) -> Result<String, ApiError> {
    if !collection.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
        return Err(ApiError::BadRequest("invalid Directus collection name".to_string()));
    }
    Ok(format!(
        "{}/items/{}",
        state.config.directus_base_url.trim_end_matches('/'),
        collection
    ))
}

fn directus_unavailable(error: reqwest::Error) -> ApiError {
    tracing::error!(?error, "Directus content bridge request failed");
    ApiError::ServiceUnavailable("Directus content bridge is unavailable".to_string())
}

fn clean_slug(input: &str) -> Result<String, ApiError> {
    let value = input.trim().to_ascii_lowercase();
    if value.is_empty() || value.chars().count() > 120 {
        return Err(ApiError::BadRequest("slug is required and must be at most 120 characters".to_string()));
    }
    if value.starts_with('-') || value.ends_with('-') || value.contains("--") {
        return Err(ApiError::BadRequest("slug format is invalid".to_string()));
    }
    if !value.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err(ApiError::BadRequest(
            "slug may only contain lowercase letters, numbers, and hyphens".to_string(),
        ));
    }
    Ok(value)
}

fn validate_lesson_blocks(blocks: &[DirectusLessonBlock]) -> Result<(), ApiError> {
    for block in blocks {
        if block.renderer_contract_version != 1 {
            return Err(ApiError::ServiceUnavailable(format!(
                "unsupported lesson block renderer contract {}",
                block.renderer_contract_version
            )));
        }
        if !LESSON_BLOCK_TYPES.contains(&block.block_type.as_str()) {
            return Err(ApiError::ServiceUnavailable(format!(
                "unsupported lesson block type {}",
                block.block_type
            )));
        }
    }
    Ok(())
}

fn validate_lab_steps(steps: &[DirectusLabStep]) -> Result<(), ApiError> {
    for step in steps {
        if step.renderer_contract_version != 1 {
            return Err(ApiError::ServiceUnavailable(format!(
                "unsupported lab step renderer contract {}",
                step.renderer_contract_version
            )));
        }
        if !LAB_STEP_TYPES.contains(&step.step_type.as_str()) {
            return Err(ApiError::ServiceUnavailable(format!(
                "unsupported lab step type {}",
                step.step_type
            )));
        }
    }
    Ok(())
}

fn validate_runtime_profile(profile: &DirectusRuntimeProfile) -> Result<(), ApiError> {
    if !LAB_RUNTIME_TYPES.contains(&profile.runtime_type.as_str()) {
        return Err(ApiError::ServiceUnavailable(format!(
            "unsupported lab runtime type {}",
            profile.runtime_type
        )));
    }
    Ok(())
}
