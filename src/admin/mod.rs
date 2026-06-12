use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use uuid::Uuid;

use crate::state::AppState;

const ADMIN_HEADER: &str = "x-karyra-admin-token";
const DATA_SOURCE_DATABASE: &str = "database";
const DATA_SOURCE_NOT_AVAILABLE: &str = "not_available";

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
enum AdminError {
    NotConfigured,
    Unauthorized,
    NotFound(&'static str),
    Database(sqlx::Error),
}

#[derive(Serialize)]
struct AdminErrorEnvelope {
    ok: bool,
    error: AdminErrorBody,
}

#[derive(Serialize)]
struct AdminErrorBody {
    code: &'static str,
    message: &'static str,
}

impl IntoResponse for AdminError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::NotConfigured => (
                StatusCode::SERVICE_UNAVAILABLE,
                "admin_not_configured",
                "Admin access is not configured.",
            ),
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "admin_unauthorized",
                "Admin access is not authorized.",
            ),
            Self::NotFound(entity) => (StatusCode::NOT_FOUND, "admin_not_found", entity),
            Self::Database(error) => {
                tracing::error!(?error, "admin database operation failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "admin_internal_error",
                    "The admin request could not be completed.",
                )
            }
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

impl From<sqlx::Error> for AdminError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/overview", get(overview))
        .route("/learners", get(learners))
        .route("/learners/:id", get(learner_detail))
        .route("/lessons", get(lessons))
        .route("/lab", get(lab))
        .route("/passports", get(passports))
        .route("/proof-ledger", get(proof_ledger))
        .route("/community-pilot", get(community_pilot))
        .route("/starknet", get(starknet))
        .route("/system", get(system))
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<(), AdminError> {
    let configured = state
        .config
        .admin_token
        .as_deref()
        .ok_or(AdminError::NotConfigured)?;
    let supplied = headers
        .get(ADMIN_HEADER)
        .and_then(|value| value.to_str().ok())
        .ok_or(AdminError::Unauthorized)?;

    // Compare fixed-length digests so token length/content are not exposed via early equality exits.
    if Sha256::digest(configured.as_bytes()) == Sha256::digest(supplied.as_bytes()) {
        // TODO(production): replace the bootstrap token with scoped RBAC and audited admin identities.
        Ok(())
    } else {
        Err(AdminError::Unauthorized)
    }
}

#[derive(Serialize)]
struct OverviewData {
    total_learners: i64,
    total_lessons: i64,
    total_lesson_completions: i64,
    total_lab_events: i64,
    total_passports: i64,
    total_proof_records: i64,
    total_participation_records: i64,
    recent_activity: Vec<Activity>,
    system_health: SystemHealth,
    starknet_status: StarknetSummary,
    data_source: &'static str,
}

#[derive(Serialize, FromRow)]
struct Activity {
    id: Uuid,
    learner_id: Uuid,
    activity_type: String,
    activity_title: String,
    source: Option<String>,
    status: String,
    timestamp: DateTime<Utc>,
}

#[derive(Serialize)]
struct SystemHealth {
    service: &'static str,
    database: &'static str,
}

#[derive(Serialize)]
struct StarknetSummary {
    status: &'static str,
    configured_networks: Vec<String>,
}

#[derive(FromRow)]
struct OverviewCounts {
    total_learners: i64,
    total_lessons: i64,
    total_lesson_completions: i64,
    total_lab_events: i64,
    total_passports: i64,
    total_proof_records: i64,
    total_participation_records: i64,
}

async fn overview(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminEnvelope<OverviewData>>, AdminError> {
    authorize(&state, &headers)?;
    let counts = sqlx::query_as::<_, OverviewCounts>(
        r#"
        select
          (select count(*) from users) as total_learners,
          (select count(distinct lesson_id) from lesson_progress) as total_lessons,
          (select count(*) from lesson_progress where status = 'completed') as total_lesson_completions,
          (
            (select count(*) from lab_attempts) +
            (select count(*) from checkpoint_results where track = 'lab') +
            (select count(*) from exam_attempts where track = 'lab')
          ) as total_lab_events,
          (select count(*) from passport_credentials) as total_passports,
          (select count(*) from proof_events) as total_proof_records,
          (select count(*) from community_workshop_registrations) as total_participation_records
        "#,
    )
    .fetch_one(&state.db)
    .await?;

    let recent_activity = sqlx::query_as::<_, Activity>(
        r#"
        select id,
               user_id as learner_id,
               event_type as activity_type,
               subject_id as activity_title,
               source_table as source,
               'recorded'::text as status,
               created_at as timestamp
        from proof_events
        order by created_at desc, id desc
        limit 20
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(success(OverviewData {
        total_learners: counts.total_learners,
        total_lessons: counts.total_lessons,
        total_lesson_completions: counts.total_lesson_completions,
        total_lab_events: counts.total_lab_events,
        total_passports: counts.total_passports,
        total_proof_records: counts.total_proof_records,
        total_participation_records: counts.total_participation_records,
        recent_activity,
        system_health: SystemHealth {
            service: "available",
            database: "connected",
        },
        starknet_status: unconfigured_starknet(),
        data_source: DATA_SOURCE_DATABASE,
    }))
}

#[derive(Deserialize)]
struct Pagination {
    limit: Option<i64>,
    offset: Option<i64>,
}

impl Pagination {
    fn values(&self) -> (i64, i64) {
        (
            self.limit.unwrap_or(50).clamp(1, 100),
            self.offset.unwrap_or(0).max(0),
        )
    }
}

#[derive(Serialize)]
struct LearnersData {
    items: Vec<LearnerSummary>,
    limit: i64,
    offset: i64,
    total: i64,
    data_source: &'static str,
}

#[derive(Serialize, FromRow)]
struct LearnerSummary {
    id: Uuid,
    display_name: Option<String>,
    email: Option<String>,
    created_at: DateTime<Utc>,
    last_seen_at: Option<DateTime<Utc>>,
    lesson_progress_summary: ProgressSummary,
    lab_progress_summary: ProgressSummary,
    passport_status_summary: PassportStatusSummary,
}

#[derive(Serialize)]
struct ProgressSummary {
    total: i64,
    completed: i64,
}

#[derive(Serialize)]
struct PassportStatusSummary {
    status: Option<String>,
    readiness_level: Option<String>,
}

#[derive(FromRow)]
struct LearnerRow {
    id: Uuid,
    display_name: Option<String>,
    email: Option<String>,
    created_at: DateTime<Utc>,
    last_seen_at: Option<DateTime<Utc>>,
    lesson_total: i64,
    lesson_completed: i64,
    lab_total: i64,
    lab_completed: i64,
    passport_status: Option<String>,
    readiness_level: Option<String>,
}

impl From<LearnerRow> for LearnerSummary {
    fn from(row: LearnerRow) -> Self {
        Self {
            id: row.id,
            display_name: row.display_name,
            email: row.email,
            created_at: row.created_at,
            last_seen_at: row.last_seen_at,
            lesson_progress_summary: ProgressSummary {
                total: row.lesson_total,
                completed: row.lesson_completed,
            },
            lab_progress_summary: ProgressSummary {
                total: row.lab_total,
                completed: row.lab_completed,
            },
            passport_status_summary: PassportStatusSummary {
                status: row.passport_status,
                readiness_level: row.readiness_level,
            },
        }
    }
}

async fn learners(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<Pagination>,
) -> Result<Json<AdminEnvelope<LearnersData>>, AdminError> {
    authorize(&state, &headers)?;
    let (limit, offset) = page.values();
    let total = sqlx::query_scalar::<_, i64>("select count(*) from users")
        .fetch_one(&state.db)
        .await?;
    let rows = sqlx::query_as::<_, LearnerRow>(
        r#"
        select u.id,
               nullif(p.display_name, '') as display_name,
               u.email::text as email,
               u.created_at,
               (select max(s.last_seen_at) from sessions s where s.user_id = u.id) as last_seen_at,
               (select count(*) from lesson_progress lp where lp.user_id = u.id) as lesson_total,
               (select count(*) from lesson_progress lp where lp.user_id = u.id and lp.status = 'completed') as lesson_completed,
               (select count(*) from lab_attempts la where la.user_id = u.id) as lab_total,
               (select count(*) from lab_attempts la where la.user_id = u.id and la.status = 'passed') as lab_completed,
               pc.issue_status as passport_status,
               pc.readiness_level
        from users u
        left join profiles p on p.user_id = u.id
        left join lateral (
          select issue_status, readiness_level
          from passport_credentials
          where user_id = u.id
          order by created_at desc
          limit 1
        ) pc on true
        order by u.created_at desc
        limit $1 offset $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    Ok(success(LearnersData {
        items: rows.into_iter().map(Into::into).collect(),
        limit,
        offset,
        total,
        data_source: DATA_SOURCE_DATABASE,
    }))
}

#[derive(Serialize)]
struct LearnerDetailData {
    profile: LearnerProfile,
    lesson_progress: Vec<LessonProgress>,
    lab_progress: Vec<LabProgress>,
    passport_summary: Option<PassportRecord>,
    evidence_proof_entries: Vec<ProofRecord>,
    participation_records: Vec<ParticipationRecord>,
    data_source: &'static str,
}

#[derive(Serialize, FromRow)]
struct LearnerProfile {
    id: Uuid,
    display_name: Option<String>,
    email: Option<String>,
    created_at: DateTime<Utc>,
    last_seen_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, FromRow)]
struct LessonProgress {
    lesson_id: String,
    level: String,
    status: String,
    progress_percent: i32,
    completed_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
}

#[derive(Serialize, FromRow)]
struct LabProgress {
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

#[derive(Serialize, FromRow)]
struct PassportRecord {
    id: Uuid,
    learner_id: Uuid,
    readiness_level: String,
    status: String,
    evidence_count: i64,
    starknet_attestation_status: String,
    updated_at: DateTime<Utc>,
}

#[derive(Serialize, FromRow)]
struct ProofRecord {
    id: Uuid,
    learner_id: Uuid,
    activity_type: String,
    activity_title: String,
    source: Option<String>,
    status: String,
    issuer_type: String,
    timestamp: DateTime<Utc>,
    related_passport_signal: bool,
    starknet_attestation_status: String,
}

#[derive(Serialize, FromRow)]
struct ParticipationRecord {
    id: Uuid,
    workshop_id: String,
    status: String,
    registered_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

async fn learner_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<AdminEnvelope<LearnerDetailData>>, AdminError> {
    authorize(&state, &headers)?;
    let profile = sqlx::query_as::<_, LearnerProfile>(
        r#"
        select u.id,
               nullif(p.display_name, '') as display_name,
               u.email::text as email,
               u.created_at,
               (select max(s.last_seen_at) from sessions s where s.user_id = u.id) as last_seen_at
        from users u
        left join profiles p on p.user_id = u.id
        where u.id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AdminError::NotFound("Learner was not found."))?;

    let lesson_progress = sqlx::query_as::<_, LessonProgress>(
        "select lesson_id, level, status, progress_percent, completed_at, updated_at from lesson_progress where user_id = $1 order by updated_at desc",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    let lab_progress = sqlx::query_as::<_, LabProgress>(
        "select id, lab_id, level, status, score, safety_score, started_at, completed_at, updated_at from lab_attempts where user_id = $1 order by updated_at desc limit 100",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    let passport_summary = latest_passport(&state, Some(id)).await?.into_iter().next();
    let evidence_proof_entries = proof_records(&state, Some(id), 100).await?;
    let participation_records = sqlx::query_as::<_, ParticipationRecord>(
        "select id, workshop_id, status, registered_at, updated_at from community_workshop_registrations where user_id = $1 order by updated_at desc limit 100",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(success(LearnerDetailData {
        profile,
        lesson_progress,
        lab_progress,
        passport_summary,
        evidence_proof_entries,
        participation_records,
        data_source: DATA_SOURCE_DATABASE,
    }))
}

#[derive(Serialize)]
struct LessonsData {
    items: Vec<LessonSummary>,
    data_source: &'static str,
    catalog_status: &'static str,
}

#[derive(Serialize, FromRow)]
struct LessonSummary {
    slug: String,
    title: Option<String>,
    status: Option<String>,
    estimated_level: Option<String>,
    completion_count: i64,
    updated_at: Option<DateTime<Utc>>,
}

async fn lessons(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminEnvelope<LessonsData>>, AdminError> {
    authorize(&state, &headers)?;
    let items = sqlx::query_as::<_, LessonSummary>(
        r#"
        select lesson_id as slug,
               null::text as title,
               null::text as status,
               min(level)::text as estimated_level,
               count(*) filter (where status = 'completed') as completion_count,
               max(updated_at) as updated_at
        from lesson_progress
        group by lesson_id
        order by lesson_id
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(success(LessonsData {
        items,
        data_source: "progress_records",
        catalog_status: "static_frontend_catalog_not_available_to_api",
    }))
}

#[derive(Serialize)]
struct LabData {
    modules: Vec<LabModuleSummary>,
    recent_lab_events: Vec<LabEvent>,
    data_source: &'static str,
}

#[derive(Serialize, FromRow)]
struct LabModuleSummary {
    module_id: String,
    name: Option<String>,
    enabled: Option<bool>,
    status: Option<String>,
    completion_count: i64,
}

#[derive(Serialize, FromRow)]
struct LabEvent {
    id: Uuid,
    learner_id: Uuid,
    module_id: String,
    status: String,
    timestamp: DateTime<Utc>,
}

async fn lab(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminEnvelope<LabData>>, AdminError> {
    authorize(&state, &headers)?;
    let modules = sqlx::query_as::<_, LabModuleSummary>(
        r#"
        select lab_id as module_id,
               null::text as name,
               null::boolean as enabled,
               null::text as status,
               count(*) filter (where status = 'passed') as completion_count
        from lab_attempts
        group by lab_id
        order by lab_id
        "#,
    )
    .fetch_all(&state.db)
    .await?;
    let recent_lab_events = sqlx::query_as::<_, LabEvent>(
        "select id, user_id as learner_id, lab_id as module_id, status, created_at as timestamp from lab_attempts order by created_at desc limit 50",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(success(LabData {
        modules,
        recent_lab_events,
        data_source: DATA_SOURCE_DATABASE,
    }))
}

#[derive(Serialize)]
struct PassportsData {
    items: Vec<PassportRecord>,
    data_source: &'static str,
}

async fn passports(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminEnvelope<PassportsData>>, AdminError> {
    authorize(&state, &headers)?;
    Ok(success(PassportsData {
        items: latest_passport(&state, None).await?,
        data_source: DATA_SOURCE_DATABASE,
    }))
}

async fn latest_passport(
    state: &AppState,
    learner_id: Option<Uuid>,
) -> Result<Vec<PassportRecord>, AdminError> {
    Ok(sqlx::query_as::<_, PassportRecord>(
        r#"
        select id,
               user_id as learner_id,
               readiness_level,
               issue_status as status,
               evidence_event_count as evidence_count,
               starknet_anchor_status as starknet_attestation_status,
               updated_at
        from passport_credentials
        where ($1::uuid is null or user_id = $1)
        order by updated_at desc
        limit 100
        "#,
    )
    .bind(learner_id)
    .fetch_all(&state.db)
    .await?)
}

#[derive(Serialize)]
struct ProofLedgerData {
    items: Vec<ProofRecord>,
    data_source: &'static str,
}

async fn proof_ledger(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminEnvelope<ProofLedgerData>>, AdminError> {
    authorize(&state, &headers)?;
    Ok(success(ProofLedgerData {
        items: proof_records(&state, None, 100).await?,
        data_source: DATA_SOURCE_DATABASE,
    }))
}

async fn proof_records(
    state: &AppState,
    learner_id: Option<Uuid>,
    limit: i64,
) -> Result<Vec<ProofRecord>, AdminError> {
    Ok(sqlx::query_as::<_, ProofRecord>(
        r#"
        select pe.id,
               pe.user_id as learner_id,
               pe.event_type as activity_type,
               pe.subject_id as activity_title,
               pe.source_table as source,
               'recorded'::text as status,
               pe.issuer as issuer_type,
               pe.created_at as timestamp,
               exists (
                 select 1 from passport_credentials pc
                 where pc.user_id = pe.user_id and pc.evidence_root = pe.event_hash
               ) as related_passport_signal,
               coalesce((
                 select pc.starknet_anchor_status
                 from passport_credentials pc
                 where pc.user_id = pe.user_id
                 order by pc.created_at desc
                 limit 1
               ), 'none') as starknet_attestation_status
        from proof_events pe
        where ($1::uuid is null or pe.user_id = $1)
        order by pe.created_at desc, pe.id desc
        limit $2
        "#,
    )
    .bind(learner_id)
    .bind(limit.clamp(1, 100))
    .fetch_all(&state.db)
    .await?)
}

#[derive(Serialize)]
struct CommunityPilotData {
    pilot_status: &'static str,
    cohorts: Vec<PilotPlaceholder>,
    sessions: Vec<PilotPlaceholder>,
    participant_count: i64,
    notes: &'static str,
    privacy_reminder: &'static str,
    data_source: &'static str,
}

#[derive(Serialize)]
struct PilotPlaceholder {
    id: String,
    status: String,
}

async fn community_pilot(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminEnvelope<CommunityPilotData>>, AdminError> {
    authorize(&state, &headers)?;
    let participant_count = sqlx::query_scalar::<_, i64>(
        "select count(distinct user_id) from community_workshop_registrations where status = 'registered'",
    )
    .fetch_one(&state.db)
    .await?;

    Ok(success(CommunityPilotData {
        pilot_status: "model_not_available",
        cohorts: Vec::new(),
        sessions: Vec::new(),
        participant_count,
        notes: "Only workshop registration signals are currently available.",
        privacy_reminder: "Export or publish only anonymized evidence; do not expose learner identity or private profile data.",
        data_source: "community_workshop_registrations",
    }))
}

#[derive(Serialize)]
struct StarknetData {
    configured_networks: Vec<String>,
    rpc_read_only_status: &'static str,
    last_checked_at: Option<DateTime<Utc>>,
    address_account_reader_status: &'static str,
    mainnet_readiness: bool,
    testnet_readiness: bool,
    status: &'static str,
    data_source: &'static str,
}

async fn starknet(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminEnvelope<StarknetData>>, AdminError> {
    authorize(&state, &headers)?;
    Ok(success(StarknetData {
        configured_networks: Vec::new(),
        rpc_read_only_status: "not_configured",
        last_checked_at: None,
        address_account_reader_status: "not_configured",
        mainnet_readiness: false,
        testnet_readiness: false,
        status: "not_configured",
        data_source: DATA_SOURCE_NOT_AVAILABLE,
    }))
}

fn unconfigured_starknet() -> StarknetSummary {
    StarknetSummary {
        status: "not_configured",
        configured_networks: Vec::new(),
    }
}

#[derive(Serialize)]
struct SystemData {
    service_name: &'static str,
    environment: String,
    app_version: &'static str,
    database_connectivity_status: &'static str,
    admin_configured: bool,
    feature_flags: FeatureFlags,
    safety_checklist: SafetyChecklist,
}

#[derive(Serialize)]
struct FeatureFlags {
    admin_api_v1_read_only: bool,
    starknet_reader: bool,
    onchain_writes: bool,
}

#[derive(Serialize)]
struct SafetyChecklist {
    no_wallet_autoconnect: bool,
    no_signature_prompt: bool,
    no_transaction_prompt: bool,
    no_private_key_handling: bool,
    no_seed_phrase_handling: bool,
}

async fn system(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminEnvelope<SystemData>>, AdminError> {
    authorize(&state, &headers)?;
    let database_connectivity_status = match sqlx::query_scalar::<_, i32>("select 1")
        .fetch_one(&state.db)
        .await
    {
        Ok(1) => "connected",
        Ok(_) => "degraded",
        Err(error) => {
            tracing::error!(?error, "admin system database health check failed");
            "unavailable"
        }
    };

    Ok(success(SystemData {
        service_name: "Karyra Spark API",
        environment: state.config.app_env.clone(),
        app_version: env!("CARGO_PKG_VERSION"),
        database_connectivity_status,
        admin_configured: state.config.admin_token.is_some(),
        feature_flags: FeatureFlags {
            admin_api_v1_read_only: true,
            starknet_reader: false,
            onchain_writes: false,
        },
        safety_checklist: SafetyChecklist {
            no_wallet_autoconnect: true,
            no_signature_prompt: true,
            no_transaction_prompt: true,
            no_private_key_handling: true,
            no_seed_phrase_handling: true,
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(token: Option<&str>) -> AppState {
        let mut config = crate::config::AppConfig::from_env();
        config.admin_token = token.map(str::to_owned);
        AppState::new(config).expect("lazy database pool")
    }

    #[tokio::test]
    async fn rejects_when_admin_is_not_configured() {
        let headers = HeaderMap::new();
        assert!(matches!(
            authorize(&state(None), &headers),
            Err(AdminError::NotConfigured)
        ));
    }

    #[tokio::test]
    async fn rejects_wrong_token_and_accepts_matching_token() {
        let state = state(Some("configured-secret"));
        let mut headers = HeaderMap::new();
        headers.insert(ADMIN_HEADER, "wrong-secret".parse().unwrap());
        assert!(matches!(
            authorize(&state, &headers),
            Err(AdminError::Unauthorized)
        ));

        headers.insert(ADMIN_HEADER, "configured-secret".parse().unwrap());
        assert!(authorize(&state, &headers).is_ok());
    }
}
