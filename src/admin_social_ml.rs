use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{admin_auth, ai_runtime, error::ApiError, moderation, state::AppState};

#[derive(Serialize)]
struct AdminEnvelope<T> {
    ok: bool,
    data: T,
    generated_at: DateTime<Utc>,
}

fn success<T>(data: T) -> Json<AdminEnvelope<T>> {
    Json(AdminEnvelope {
        ok: true,
        data,
        generated_at: Utc::now(),
    })
}

#[derive(Debug)]
enum AdminMlError {
    NotConfigured,
    Unauthorized,
    BadRequest(String),
    NotFound(&'static str),
    Database(sqlx::Error),
    Internal,
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

impl IntoResponse for AdminMlError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::NotConfigured => (
                StatusCode::SERVICE_UNAVAILABLE,
                "admin_not_configured",
                "Admin access is not configured.".to_string(),
            ),
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "admin_unauthorized",
                "Admin access is not authorized.".to_string(),
            ),
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "admin_bad_request", message),
            Self::NotFound(entity) => (StatusCode::NOT_FOUND, "admin_not_found", entity.to_string()),
            Self::Database(error) => {
                tracing::error!(?error, "admin social ML database operation failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "admin_internal_error",
                    "The admin social ML request could not be completed.".to_string(),
                )
            }
            Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "admin_internal_error",
                "The admin social ML request could not be completed.".to_string(),
            ),
        };

        (
            status,
            Json(AdminErrorEnvelope {
                ok: false,
                error: AdminErrorBody { code, message },
            }),
        )
            .into_response()
    }
}

impl From<sqlx::Error> for AdminMlError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

impl From<ApiError> for AdminMlError {
    fn from(value: ApiError) -> Self {
        match value {
            ApiError::Unauthorized => Self::Unauthorized,
            ApiError::BadRequest(message) => Self::BadRequest(message),
            ApiError::ServiceUnavailable(_) => Self::NotConfigured,
            error => {
                tracing::error!(?error, "admin social ML authorization/runtime failed");
                Self::Internal
            }
        }
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/scope", get(scope))
        .route("/signals", get(signals))
        .route("/scan", post(scan_target))
        .route("/scan-batch", post(scan_batch))
        .route("/signals/:signal_id/mark-reviewed", post(mark_signal_reviewed))
}

#[derive(Serialize)]
struct ScopeData {
    module: &'static str,
    phase: &'static str,
    routes: Vec<&'static str>,
    statuses: Vec<&'static str>,
    categories: Vec<&'static str>,
    auto_action: bool,
    auth_model: &'static str,
}

async fn scope(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminEnvelope<ScopeData>>, AdminMlError> {
    admin_auth::authorize_with_capability(&state, &headers, "moderation_read").await?;
    Ok(success(ScopeData {
        module: module_path!(),
        phase: "ml-moderation-signal-pipeline",
        routes: vec![
            "GET /api/admin/social/ml/scope",
            "GET /api/admin/social/ml/signals",
            "POST /api/admin/social/ml/scan",
            "POST /api/admin/social/ml/scan-batch",
            "POST /api/admin/social/ml/signals/:signal_id/mark-reviewed",
        ],
        statuses: vec!["clean", "needs_review", "high_risk", "blocked_pending_review"],
        categories: vec![
            "spam",
            "harassment",
            "hate",
            "sexual_content",
            "violence",
            "self_harm",
            "illegal_unsafe_instruction",
            "scam_fraud",
            "private_data_leak",
            "crypto_scam_wallet_drain",
            "off_topic_low_quality",
        ],
        auto_action: false,
        auth_model: "legacy superadmin root plus delegated admin ML moderation capability",
    }))
}

#[derive(Debug, Deserialize)]
struct SignalQuery {
    target_type: Option<String>,
    status: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

impl SignalQuery {
    fn paging(&self) -> (i64, i64) {
        (
            self.limit.unwrap_or(50).clamp(1, 100),
            self.offset.unwrap_or(0).max(0),
        )
    }
}

#[derive(Debug, Serialize, FromRow)]
struct SignalRow {
    id: Uuid,
    target_type: String,
    target_id: Uuid,
    target_owner_user_id: Option<Uuid>,
    source: String,
    status: String,
    decision: String,
    categories: Vec<String>,
    severity: String,
    score: f32,
    summary: String,
    recommendation: String,
    moderation_event_id: Option<Uuid>,
    model_run_ids: Vec<Uuid>,
    created_by_kind: String,
    created_by_user_id: Option<Uuid>,
    reviewed_by_user_id: Option<Uuid>,
    reviewed_at: Option<DateTime<Utc>>,
    review_note: String,
    metadata: Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct SignalListData {
    items: Vec<SignalRow>,
    total: i64,
    limit: i64,
    offset: i64,
    data_source: &'static str,
}

async fn signals(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SignalQuery>,
) -> Result<Json<AdminEnvelope<SignalListData>>, AdminMlError> {
    admin_auth::authorize_with_capability(&state, &headers, "moderation_read").await?;
    let (limit, offset) = query.paging();
    let target_type = query
        .target_type
        .as_deref()
        .map(normalize_target_type)
        .transpose()?;
    let status = query.status.as_deref().map(normalize_signal_status).transpose()?;

    let total = sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)
        from social_moderation_ml_signals
        where ($1::text is null or target_type = $1)
          and ($2::text is null or status = $2)
        "#,
    )
    .bind(target_type.as_deref())
    .bind(status.as_deref())
    .fetch_one(&state.db)
    .await?;

    let items = sqlx::query_as::<_, SignalRow>(SIGNAL_SELECT_SQL)
        .bind(target_type.as_deref())
        .bind(status.as_deref())
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?;

    Ok(success(SignalListData {
        items,
        total,
        limit,
        offset,
        data_source: "database",
    }))
}

const SIGNAL_SELECT_SQL: &str = r#"
select id,
       target_type,
       target_id,
       target_owner_user_id,
       source,
       status,
       decision,
       categories,
       severity,
       score::float4 as score,
       summary,
       recommendation,
       moderation_event_id,
       model_run_ids,
       created_by_kind,
       created_by_user_id,
       reviewed_by_user_id,
       reviewed_at,
       review_note,
       metadata,
       created_at,
       updated_at
from social_moderation_ml_signals
where ($1::text is null or target_type = $1)
  and ($2::text is null or status = $2)
order by created_at desc, id desc
limit $3 offset $4
"#;

#[derive(Debug, Deserialize)]
struct ScanRequest {
    target_type: String,
    target_id: Uuid,
    use_local_ai: Option<bool>,
    use_external_fallback: Option<bool>,
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ScanBatchRequest {
    target_type: String,
    status: Option<String>,
    limit: Option<i64>,
    use_local_ai: Option<bool>,
    use_external_fallback: Option<bool>,
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MarkReviewedRequest {
    note: Option<String>,
}

#[derive(Debug, Serialize)]
struct ScanData {
    signal: SignalRow,
    rule_decision: SignalDecisionData,
    local_ai: Option<SignalAiRunData>,
    external_ai: Option<SignalAiRunData>,
    provider_errors: Vec<String>,
    auto_action: bool,
}

#[derive(Debug, Serialize)]
struct BatchScanData {
    items: Vec<SignalRow>,
    scanned_count: i64,
    target_type: String,
    status_filter: Option<String>,
    auto_action: bool,
}

#[derive(Debug, Serialize)]
struct SignalDecisionData {
    decision: String,
    categories: Vec<String>,
    score: f32,
    severity: String,
    summary: String,
}

#[derive(Debug, Serialize, Clone)]
struct SignalAiRunData {
    id: Option<Uuid>,
    provider: String,
    model: String,
    decision: String,
    categories: Vec<String>,
    score: f32,
    summary: String,
    latency_ms: i64,
}

#[derive(Debug, FromRow)]
struct TargetContentRow {
    id: Uuid,
    owner_user_id: Uuid,
    body: String,
    status: String,
}

async fn scan_target(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ScanRequest>,
) -> Result<(StatusCode, Json<AdminEnvelope<ScanData>>), AdminMlError> {
    let actor = admin_auth::authorize_with_capability(&state, &headers, "ml_moderation_manage").await?;
    let target_type = normalize_target_type(&payload.target_type)?;
    let note = clean_note(payload.note.as_deref())?;
    let data = scan_one(
        &state,
        &actor,
        &target_type,
        payload.target_id,
        payload.use_local_ai.unwrap_or(true),
        payload.use_external_fallback.unwrap_or(false),
        note.as_deref(),
    )
    .await?;

    Ok((StatusCode::CREATED, success(data)))
}

async fn scan_batch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ScanBatchRequest>,
) -> Result<(StatusCode, Json<AdminEnvelope<BatchScanData>>), AdminMlError> {
    let actor = admin_auth::authorize_with_capability(&state, &headers, "ml_moderation_manage").await?;
    let target_type = normalize_target_type(&payload.target_type)?;
    let status_filter = payload
        .status
        .as_deref()
        .map(normalize_content_status)
        .transpose()?;
    let limit = payload.limit.unwrap_or(25).clamp(1, 50);
    let note = clean_note(payload.note.as_deref())?;
    let target_ids = fetch_batch_target_ids(&state, &target_type, status_filter.as_deref(), limit).await?;
    let mut items = Vec::with_capacity(target_ids.len());

    for target_id in target_ids {
        let data = scan_one(
            &state,
            &actor,
            &target_type,
            target_id,
            payload.use_local_ai.unwrap_or(true),
            payload.use_external_fallback.unwrap_or(false),
            note.as_deref(),
        )
        .await?;
        items.push(data.signal);
    }

    let scanned_count = items.len() as i64;
    admin_auth::audit(
        &state,
        &actor,
        "ml_moderation_batch_scan",
        "social_moderation_ml_signal",
        None,
        None,
        &actor.capabilities,
        "ML moderation batch scan completed without automatic content action.",
        json!({
            "target_type": target_type,
            "status_filter": status_filter,
            "scanned_count": scanned_count,
            "auto_action": false,
        }),
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        success(BatchScanData {
            items,
            scanned_count,
            target_type,
            status_filter,
            auto_action: false,
        }),
    ))
}

async fn mark_signal_reviewed(
    Path(signal_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<MarkReviewedRequest>,
) -> Result<Json<AdminEnvelope<SignalRow>>, AdminMlError> {
    let actor = admin_auth::authorize_with_capability(&state, &headers, "ml_moderation_manage").await?;
    let note = clean_note(payload.note.as_deref())?.unwrap_or_default();
    let signal = sqlx::query_as::<_, SignalRow>(
        r#"
        update social_moderation_ml_signals
        set reviewed_by_user_id = $2,
            reviewed_at = now(),
            review_note = $3,
            updated_at = now()
        where id = $1
        returning id,
                  target_type,
                  target_id,
                  target_owner_user_id,
                  source,
                  status,
                  decision,
                  categories,
                  severity,
                  score::float4 as score,
                  summary,
                  recommendation,
                  moderation_event_id,
                  model_run_ids,
                  created_by_kind,
                  created_by_user_id,
                  reviewed_by_user_id,
                  reviewed_at,
                  review_note,
                  metadata,
                  created_at,
                  updated_at
        "#,
    )
    .bind(signal_id)
    .bind(actor.actor_user_id)
    .bind(&note)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AdminMlError::NotFound("ML moderation signal was not found."))?;

    admin_auth::audit(
        &state,
        &actor,
        "ml_moderation_signal_reviewed",
        &signal.target_type,
        signal.target_owner_user_id,
        Some(signal.target_id),
        &actor.capabilities,
        "ML moderation signal was marked reviewed by a human operator.",
        json!({
            "signal_id": signal.id,
            "status": signal.status,
            "decision": signal.decision,
            "note": note,
            "auto_action": false,
        }),
    )
    .await?;

    Ok(success(signal))
}

async fn scan_one(
    state: &AppState,
    actor: &admin_auth::AdminContext,
    target_type: &str,
    target_id: Uuid,
    use_local_ai: bool,
    use_external_fallback: bool,
    note: Option<&str>,
) -> Result<ScanData, AdminMlError> {
    let target = fetch_target(state, target_type, target_id).await?;
    let rule = moderation::evaluate_text(&target.body);
    let rule_score = rule.score.unwrap_or(0.0).clamp(0.0, 1.0);
    let mut provider_errors = Vec::<String>::new();
    let mut model_run_ids = Vec::<Uuid>::new();

    let local_ai = if use_local_ai {
        run_local_ai(state, &target.body, &mut model_run_ids, &mut provider_errors).await
    } else {
        None
    };

    let should_external = use_external_fallback && should_use_external(&rule, local_ai.as_ref());
    let external_ai = if should_external {
        run_external_ai(state, &target.body, &mut model_run_ids, &mut provider_errors).await
    } else {
        None
    };

    let final_decision = choose_decision(&rule, local_ai.as_ref(), external_ai.as_ref());
    let categories = merge_categories(&rule.categories, local_ai.as_ref(), external_ai.as_ref());
    let final_score = [
        Some(rule_score),
        local_ai.as_ref().map(|item| item.score),
        external_ai.as_ref().map(|item| item.score),
    ]
    .into_iter()
    .flatten()
    .fold(0.0_f32, f32::max)
    .clamp(0.0, 1.0);
    let signal_status = signal_status_for(&final_decision, final_score);
    let severity = severity_for(final_score, &final_decision);
    let source = source_for(local_ai.as_ref(), external_ai.as_ref());
    let summary = summary_for(&rule, local_ai.as_ref(), external_ai.as_ref(), &provider_errors);
    let recommendation = recommendation_for(signal_status);
    let moderation_event_id = insert_moderation_event(
        state,
        target_type,
        target.id,
        target.owner_user_id,
        &final_decision,
        &categories,
        severity,
        final_score,
        source,
        &summary,
        note,
    )
    .await?;

    let signal = insert_signal(
        state,
        actor,
        target_type,
        &target,
        source,
        signal_status,
        &final_decision,
        &categories,
        severity,
        final_score,
        &summary,
        recommendation,
        moderation_event_id,
        &model_run_ids,
        local_ai.as_ref(),
        external_ai.as_ref(),
        &provider_errors,
        note,
    )
    .await?;

    update_target_moderation_columns(
        state,
        target_type,
        target.id,
        moderation_status_for(&final_decision),
        &final_decision,
        &categories,
        final_score,
        source,
        recommendation,
    )
    .await?;

    admin_auth::audit(
        state,
        actor,
        "ml_moderation_signal_create",
        target_type,
        Some(target.owner_user_id),
        Some(target.id),
        &actor.capabilities,
        "ML moderation signal was created without automatic content action.",
        json!({
            "signal_id": signal.id,
            "signal_status": signal.status,
            "decision": signal.decision,
            "score": signal.score,
            "categories": signal.categories,
            "source": signal.source,
            "target_content_status": target.status,
            "provider_errors": provider_errors,
            "auto_action": false,
        }),
    )
    .await?;

    Ok(ScanData {
        signal,
        rule_decision: SignalDecisionData {
            decision: rule.decision.to_string(),
            categories: rule.categories,
            score: rule_score,
            severity: rule.severity.to_string(),
            summary: rule.admin_summary,
        },
        local_ai,
        external_ai,
        provider_errors,
        auto_action: false,
    })
}

async fn fetch_target(
    state: &AppState,
    target_type: &str,
    target_id: Uuid,
) -> Result<TargetContentRow, AdminMlError> {
    let row = match target_type {
        "post" => sqlx::query_as::<_, TargetContentRow>(
            r#"
            select id,
                   author_user_id as owner_user_id,
                   body,
                   status
            from social_posts
            where id = $1
            "#,
        )
        .bind(target_id)
        .fetch_optional(&state.db)
        .await?,
        "comment" => sqlx::query_as::<_, TargetContentRow>(
            r#"
            select id,
                   author_user_id as owner_user_id,
                   body,
                   status
            from social_comments
            where id = $1
            "#,
        )
        .bind(target_id)
        .fetch_optional(&state.db)
        .await?,
        _ => None,
    };

    row.ok_or(AdminMlError::NotFound("Social moderation target was not found."))
}

async fn fetch_batch_target_ids(
    state: &AppState,
    target_type: &str,
    status: Option<&str>,
    limit: i64,
) -> Result<Vec<Uuid>, AdminMlError> {
    let rows = match target_type {
        "post" => {
            sqlx::query_scalar::<_, Uuid>(
                r#"
                select id
                from social_posts
                where ($1::text is null or status = $1)
                order by created_at desc, id desc
                limit $2
                "#,
            )
            .bind(status)
            .bind(limit)
            .fetch_all(&state.db)
            .await?
        }
        "comment" => {
            sqlx::query_scalar::<_, Uuid>(
                r#"
                select id
                from social_comments
                where ($1::text is null or status = $1)
                order by created_at desc, id desc
                limit $2
                "#,
            )
            .bind(status)
            .bind(limit)
            .fetch_all(&state.db)
            .await?
        }
        _ => Vec::new(),
    };
    Ok(rows)
}

async fn run_local_ai(
    state: &AppState,
    text: &str,
    model_run_ids: &mut Vec<Uuid>,
    provider_errors: &mut Vec<String>,
) -> Option<SignalAiRunData> {
    match ai_runtime::provider(state, "ollama_local").await {
        Ok(Some(provider)) if provider.enabled => match ai_runtime::ollama_moderate_text(state, &provider, text).await {
            Ok(output) => Some(record_ai_run(state, output, model_run_ids, provider_errors).await),
            Err(error) => {
                provider_errors.push(format!("local_ai: {error}"));
                None
            }
        },
        Ok(_) => None,
        Err(error) => {
            provider_errors.push(format!("local_ai_config: {error}"));
            None
        }
    }
}

async fn run_external_ai(
    state: &AppState,
    text: &str,
    model_run_ids: &mut Vec<Uuid>,
    provider_errors: &mut Vec<String>,
) -> Option<SignalAiRunData> {
    match ai_runtime::provider(state, "openai_moderation").await {
        Ok(Some(provider)) if provider.enabled => match ai_runtime::openai_moderate_text(state, &provider, text).await {
            Ok(output) => Some(record_ai_run(state, output, model_run_ids, provider_errors).await),
            Err(error) => {
                provider_errors.push(format!("external_ai: {error}"));
                None
            }
        },
        Ok(_) => None,
        Err(error) => {
            provider_errors.push(format!("external_ai_config: {error}"));
            None
        }
    }
}

async fn record_ai_run(
    state: &AppState,
    output: ai_runtime::AiRunOutput,
    model_run_ids: &mut Vec<Uuid>,
    provider_errors: &mut Vec<String>,
) -> SignalAiRunData {
    let run_id = match ai_runtime::log_model_run(state, None, &output, "social_text").await {
        Ok(id) => {
            model_run_ids.push(id);
            Some(id)
        }
        Err(error) => {
            provider_errors.push(format!("model_run_log: {error}"));
            None
        }
    };

    SignalAiRunData {
        id: run_id,
        provider: output.provider,
        model: output.model,
        decision: normalize_ai_decision(&output.decision).to_string(),
        categories: output.categories,
        score: output.score.clamp(0.0, 1.0),
        summary: output.summary,
        latency_ms: output.latency_ms,
    }
}

async fn insert_moderation_event(
    state: &AppState,
    target_type: &str,
    target_id: Uuid,
    owner_user_id: Uuid,
    decision: &str,
    categories: &[String],
    severity: &str,
    score: f32,
    source: &str,
    summary: &str,
    note: Option<&str>,
) -> Result<Uuid, AdminMlError> {
    let event_id = Uuid::new_v4();
    sqlx::query(
        r#"
        insert into moderation_events (
          id, actor_user_id, target_type, target_id, target_owner_user_id,
          event_type, decision, categories, severity, score, source,
          user_message, admin_summary, metadata
        ) values ($1, null, $2, $3, $4, 'ai_review', $5, $6, $7, $8::numeric, $9, '', $10, $11)
        "#,
    )
    .bind(event_id)
    .bind(target_type)
    .bind(target_id)
    .bind(owner_user_id)
    .bind(decision)
    .bind(categories.to_vec())
    .bind(severity)
    .bind(format!("{score:.5}"))
    .bind(source)
    .bind(summary)
    .bind(json!({"engine": "ml_signal_pipeline", "auto_action": false, "note": note}))
    .execute(&state.db)
    .await?;
    Ok(event_id)
}

async fn insert_signal(
    state: &AppState,
    actor: &admin_auth::AdminContext,
    target_type: &str,
    target: &TargetContentRow,
    source: &str,
    status: &str,
    decision: &str,
    categories: &[String],
    severity: &str,
    score: f32,
    summary: &str,
    recommendation: &str,
    moderation_event_id: Uuid,
    model_run_ids: &[Uuid],
    local_ai: Option<&SignalAiRunData>,
    external_ai: Option<&SignalAiRunData>,
    provider_errors: &[String],
    note: Option<&str>,
) -> Result<SignalRow, AdminMlError> {
    Ok(sqlx::query_as::<_, SignalRow>(
        r#"
        insert into social_moderation_ml_signals (
          id, target_type, target_id, target_owner_user_id, source, status,
          decision, categories, severity, score, summary, recommendation,
          moderation_event_id, model_run_ids, created_by_kind, created_by_user_id, metadata
        ) values ($1, $2, $3, $4, $5, $6,
                  $7, $8, $9, $10::numeric, $11, $12,
                  $13, $14, $15, $16, $17)
        returning id,
                  target_type,
                  target_id,
                  target_owner_user_id,
                  source,
                  status,
                  decision,
                  categories,
                  severity,
                  score::float4 as score,
                  summary,
                  recommendation,
                  moderation_event_id,
                  model_run_ids,
                  created_by_kind,
                  created_by_user_id,
                  reviewed_by_user_id,
                  reviewed_at,
                  review_note,
                  metadata,
                  created_at,
                  updated_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(target_type)
    .bind(target.id)
    .bind(target.owner_user_id)
    .bind(source)
    .bind(status)
    .bind(decision)
    .bind(categories.to_vec())
    .bind(severity)
    .bind(format!("{score:.5}"))
    .bind(summary)
    .bind(recommendation)
    .bind(moderation_event_id)
    .bind(model_run_ids.to_vec())
    .bind(&actor.actor_kind)
    .bind(actor.actor_user_id)
    .bind(json!({
        "target_content_status": target.status,
        "local_ai": local_ai,
        "external_ai": external_ai,
        "provider_errors": provider_errors,
        "note": note,
        "auto_action": false,
    }))
    .fetch_one(&state.db)
    .await?)
}

async fn update_target_moderation_columns(
    state: &AppState,
    target_type: &str,
    target_id: Uuid,
    moderation_status: &str,
    decision: &str,
    categories: &[String],
    score: f32,
    source: &str,
    message: &str,
) -> Result<(), AdminMlError> {
    match target_type {
        "post" => {
            sqlx::query(
                r#"
                update social_posts
                set moderation_status = $2,
                    moderation_decision = $3,
                    moderation_categories = $4,
                    moderation_score = $5::numeric,
                    moderation_checked_at = now(),
                    moderation_source = $6,
                    moderation_message = $7,
                    updated_at = now()
                where id = $1
                "#,
            )
            .bind(target_id)
            .bind(moderation_status)
            .bind(decision)
            .bind(categories.to_vec())
            .bind(format!("{score:.5}"))
            .bind(source)
            .bind(message)
            .execute(&state.db)
            .await?;
        }
        "comment" => {
            sqlx::query(
                r#"
                update social_comments
                set moderation_status = $2,
                    moderation_decision = $3,
                    moderation_categories = $4,
                    moderation_score = $5::numeric,
                    moderation_checked_at = now(),
                    moderation_source = $6,
                    moderation_message = $7,
                    updated_at = now()
                where id = $1
                "#,
            )
            .bind(target_id)
            .bind(moderation_status)
            .bind(decision)
            .bind(categories.to_vec())
            .bind(format!("{score:.5}"))
            .bind(source)
            .bind(message)
            .execute(&state.db)
            .await?;
        }
        _ => {}
    }
    Ok(())
}

fn should_use_external(rule: &moderation::ModerationOutcome, local_ai: Option<&SignalAiRunData>) -> bool {
    if rule.is_block() || rule.is_review() {
        return true;
    }
    match local_ai {
        Some(run) => run.decision != "allow" || run.score >= 0.55,
        None => false,
    }
}

fn choose_decision(
    rule: &moderation::ModerationOutcome,
    local_ai: Option<&SignalAiRunData>,
    external_ai: Option<&SignalAiRunData>,
) -> String {
    let mut decision = normalize_ai_decision(rule.decision).to_string();
    for item in [local_ai, external_ai].into_iter().flatten() {
        decision = max_decision(&decision, &item.decision).to_string();
    }
    decision
}

fn max_decision<'a>(current: &'a str, next: &'a str) -> &'a str {
    if decision_rank(next) > decision_rank(current) {
        next
    } else {
        current
    }
}

fn decision_rank(value: &str) -> i32 {
    match value {
        "allow" => 0,
        "review" => 1,
        "block" => 2,
        _ => 1,
    }
}

fn normalize_ai_decision(input: &str) -> &'static str {
    match input.trim() {
        "allow" | "assist" => "allow",
        "block" => "block",
        _ => "review",
    }
}

fn merge_categories(
    rule_categories: &[String],
    local_ai: Option<&SignalAiRunData>,
    external_ai: Option<&SignalAiRunData>,
) -> Vec<String> {
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

fn signal_status_for(decision: &str, score: f32) -> &'static str {
    match decision {
        "block" => "blocked_pending_review",
        "review" if score >= 0.75 => "high_risk",
        "review" => "needs_review",
        _ => "clean",
    }
}

fn severity_for(score: f32, decision: &str) -> &'static str {
    if decision == "block" || score >= 0.9 {
        "critical"
    } else if score >= 0.75 {
        "high"
    } else if score >= 0.55 {
        "medium"
    } else {
        "low"
    }
}

fn moderation_status_for(decision: &str) -> &'static str {
    match decision {
        "block" => "blocked",
        "review" => "pending_review",
        _ => "allowed",
    }
}

fn source_for(local_ai: Option<&SignalAiRunData>, external_ai: Option<&SignalAiRunData>) -> &'static str {
    match (local_ai, external_ai) {
        (Some(_), Some(_)) => "combined",
        (Some(_), None) => "local_ai",
        (None, Some(_)) => "external_ai",
        (None, None) => "rules",
    }
}

fn summary_for(
    rule: &moderation::ModerationOutcome,
    local_ai: Option<&SignalAiRunData>,
    external_ai: Option<&SignalAiRunData>,
    provider_errors: &[String],
) -> String {
    let mut parts = Vec::<String>::new();
    if !rule.admin_summary.is_empty() {
        parts.push(rule.admin_summary.clone());
    } else {
        parts.push("Rule scan did not find a blocking policy match.".to_string());
    }
    if let Some(run) = local_ai {
        parts.push(format!("Local AI: {} ({:.2}).", run.decision, run.score));
    }
    if let Some(run) = external_ai {
        parts.push(format!("External AI: {} ({:.2}).", run.decision, run.score));
    }
    if !provider_errors.is_empty() {
        parts.push(format!("Provider warnings: {}.", provider_errors.join(" | ")));
    }
    parts.join(" ")
}

fn recommendation_for(status: &str) -> &'static str {
    match status {
        "blocked_pending_review" => "High-confidence risk. Human moderator should review before any remove/hide action.",
        "high_risk" => "Prioritize human review. Do not auto-remove.",
        "needs_review" => "Queue for human review. No automatic content action was taken.",
        _ => "No ML safety objection found. Human review remains available.",
    }
}

fn normalize_target_type(input: &str) -> Result<String, AdminMlError> {
    match input.trim() {
        "post" => Ok("post".to_string()),
        "comment" => Ok("comment".to_string()),
        _ => Err(AdminMlError::BadRequest(
            "target_type must be post or comment".to_string(),
        )),
    }
}

fn normalize_signal_status(input: &str) -> Result<String, AdminMlError> {
    match input.trim() {
        "clean" | "needs_review" | "high_risk" | "blocked_pending_review" => {
            Ok(input.trim().to_string())
        }
        _ => Err(AdminMlError::BadRequest(
            "status must be clean, needs_review, high_risk, or blocked_pending_review".to_string(),
        )),
    }
}

fn normalize_content_status(input: &str) -> Result<String, AdminMlError> {
    match input.trim() {
        "published" | "hidden" | "removed" | "deleted" => Ok(input.trim().to_string()),
        _ => Err(AdminMlError::BadRequest(
            "content status must be published, hidden, removed, or deleted".to_string(),
        )),
    }
}

fn clean_note(input: Option<&str>) -> Result<Option<String>, AdminMlError> {
    let Some(raw) = input else {
        return Ok(None);
    };
    let value = raw.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > 1000 {
        return Err(AdminMlError::BadRequest("note is too long".to_string()));
    }
    if value.chars().any(char::is_control) {
        return Err(AdminMlError::BadRequest(
            "note cannot contain control characters".to_string(),
        ));
    }
    Ok(Some(value.to_string()))
}
