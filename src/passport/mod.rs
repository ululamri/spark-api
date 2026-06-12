use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    auth::session::require_current_user, error::ApiError, progress::normalize_level,
    state::AppState,
};

const PASSPORT_SCHEMA_VERSION: &str = "spark-passport-v1";
const PASSPORT_ISSUER: &str = "karyra-spark-api";
const VERIFICATION_TIER: &str = "system_verified_mvp";
const STARKNET_ANCHOR_STATUS: &str = "not_ready";

#[derive(Serialize)]
struct PassportScopeResponse {
    product: &'static str,
    proof_output: &'static str,
    current_phase: &'static str,
    implemented_now: Vec<&'static str>,
    backend_role: Vec<&'static str>,
    grant_scope_hold: Vec<&'static str>,
}

#[derive(Debug, Deserialize)]
pub struct IssuePassportRequest {
    pub readiness_level: Option<String>,
}

#[derive(Debug, Serialize)]
struct PassportEligibilityResponse {
    user_id: Uuid,
    eligible: bool,
    highest_eligible_level: Option<String>,
    evidence_root: Option<String>,
    evidence_event_count: i64,
    levels: Vec<LevelEligibilityResponse>,
    note: &'static str,
}

#[derive(Debug, Serialize)]
struct LevelEligibilityResponse {
    level: String,
    eligible: bool,
    proof_of_learning: bool,
    proof_of_practice: bool,
    proof_of_safety: bool,
    proof_of_readiness: bool,
    counts: LevelEvidenceCounts,
    evidence_root: Option<String>,
    evidence_event_count: i64,
    missing: Vec<String>,
}

#[derive(Debug, Serialize)]
struct LevelEvidenceCounts {
    lesson_completed_count: i64,
    core_checkpoint_passed_count: i64,
    lab_checkpoint_passed_count: i64,
    core_exam_passed_count: i64,
    lab_exam_passed_count: i64,
    lab_attempt_passed_count: i64,
    safety_passed_count: i64,
}

#[derive(Debug, FromRow)]
struct LevelEvidenceRow {
    lesson_completed_count: i64,
    core_checkpoint_passed_count: i64,
    lab_checkpoint_passed_count: i64,
    core_exam_passed_count: i64,
    lab_exam_passed_count: i64,
    lab_attempt_passed_count: i64,
    safety_passed_count: i64,
    evidence_event_count: i64,
    evidence_root: Option<String>,
}

#[derive(Debug, Serialize)]
struct CurrentPassportResponse {
    credential: Option<PassportCredentialResponse>,
}

#[derive(Debug, Serialize)]
struct PassportCredentialResponse {
    id: Uuid,
    user_id: Uuid,
    readiness_level: String,
    verification_tier: String,
    issue_status: String,
    evidence_root: Option<String>,
    evidence_event_count: i64,
    starknet_anchor_status: String,
    schema_version: String,
    issuer: String,
    credential_hash: Option<String>,
    issued_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    nft_ready: bool,
    on_chain: bool,
}

#[derive(Debug, FromRow)]
struct PassportCredentialRow {
    id: Uuid,
    user_id: Uuid,
    readiness_level: String,
    verification_tier: String,
    issue_status: String,
    evidence_root: Option<String>,
    evidence_event_count: i64,
    starknet_anchor_status: String,
    schema_version: String,
    issuer: String,
    credential_hash: Option<String>,
    issued_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/scope", get(scope))
        .route("/me", get(my_passport))
        .route("/me/eligibility", get(my_eligibility))
        .route("/me/issue", post(issue_my_passport))
        .route("/me/revoke", post(revoke_my_passport))
}

async fn scope() -> Json<PassportScopeResponse> {
    Json(PassportScopeResponse {
        product: "Passport Spark",
        proof_output: "Proof-of-Readiness",
        current_phase: "passport-credential-api",
        implemented_now: vec![
            "eligibility-engine",
            "system-evidence-root-check",
            "passport-credential-issuance",
            "credential-lifecycle-issued-revoked-superseded",
            "nft-ready-offchain-record",
        ],
        backend_role: vec![
            "derive readiness from system proof records",
            "issue backend Passport credential",
            "preserve evidence root for future Starknet anchor",
            "keep Passport separate from user Profile",
        ],
        grant_scope_hold: vec![
            "Cairo PassportRegistry",
            "StarknetKit wallet readiness",
            "Sepolia testing",
            "Mainnet anchor",
            "NFT or non-transferable Passport Badge mint",
            "public verifier",
        ],
    })
}

async fn my_eligibility(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PassportEligibilityResponse>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let eligibility = load_passport_eligibility(&state, user.id).await?;
    Ok(Json(eligibility))
}

async fn my_passport(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CurrentPassportResponse>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let credential = latest_passport_credential(&state, user.id).await?;

    Ok(Json(CurrentPassportResponse {
        credential: credential.map(PassportCredentialResponse::from),
    }))
}

async fn issue_my_passport(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<IssuePassportRequest>,
) -> Result<(StatusCode, Json<PassportCredentialResponse>), ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let eligibility = load_passport_eligibility(&state, user.id).await?;

    let highest_level = eligibility
        .highest_eligible_level
        .clone()
        .ok_or_else(|| ApiError::BadRequest("Passport is not eligible to issue yet".to_string()))?;

    let target_level = match payload.readiness_level.as_deref() {
        Some(level) => normalize_level(level)?,
        None => highest_level.clone(),
    };

    if readiness_rank(&target_level) > readiness_rank(&highest_level) {
        return Err(ApiError::BadRequest(format!(
            "requested readiness level {target_level} is higher than current eligible level {highest_level}"
        )));
    }

    let level_state = eligibility
        .levels
        .iter()
        .find(|level| level.level == target_level)
        .ok_or_else(|| {
            ApiError::BadRequest("requested readiness level is not available".to_string())
        })?;

    if !level_state.eligible {
        return Err(ApiError::BadRequest(format!(
            "Passport is not eligible for {target_level} level yet"
        )));
    }

    let evidence_root = eligibility.evidence_root.clone().ok_or_else(|| {
        ApiError::BadRequest(
            "Passport requires a backend evidence root before issuance".to_string(),
        )
    })?;
    let credential_hash = credential_hash(
        user.id,
        &target_level,
        &evidence_root,
        eligibility.evidence_event_count,
    );

    sqlx::query(
        r#"
        update passport_credentials
        set issue_status = 'superseded', updated_at = now()
        where user_id = $1 and issue_status = 'issued'
        "#,
    )
    .bind(user.id)
    .execute(&state.db)
    .await?;

    let row = sqlx::query_as::<_, PassportCredentialRow>(
        r#"
        insert into passport_credentials (
            user_id,
            readiness_level,
            verification_tier,
            issue_status,
            evidence_root,
            evidence_event_count,
            starknet_anchor_status,
            schema_version,
            issuer,
            credential_hash,
            issued_at,
            payload
        )
        values ($1, $2, $3, 'issued', $4, $5, $6, $7, $8, $9, now(), $10)
        returning id,
                  user_id,
                  readiness_level,
                  verification_tier,
                  issue_status,
                  evidence_root,
                  evidence_event_count,
                  starknet_anchor_status,
                  schema_version,
                  issuer,
                  credential_hash,
                  issued_at,
                  revoked_at,
                  created_at,
                  updated_at
        "#,
    )
    .bind(user.id)
    .bind(target_level.clone())
    .bind(VERIFICATION_TIER)
    .bind(evidence_root.clone())
    .bind(eligibility.evidence_event_count)
    .bind(STARKNET_ANCHOR_STATUS)
    .bind(PASSPORT_SCHEMA_VERSION)
    .bind(PASSPORT_ISSUER)
    .bind(credential_hash)
    .bind(json!({
        "source": "passport.eligibility_engine",
        "readiness_level": target_level,
        "evidence_root": evidence_root,
        "evidence_event_count": eligibility.evidence_event_count,
        "proof_of_learning": level_state.proof_of_learning,
        "proof_of_practice": level_state.proof_of_practice,
        "proof_of_safety": level_state.proof_of_safety,
        "proof_of_readiness": level_state.proof_of_readiness,
        "on_chain": false,
        "nft_minted": false
    }))
    .fetch_one(&state.db)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(PassportCredentialResponse::from(row)),
    ))
}

async fn revoke_my_passport(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PassportCredentialResponse>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let row = sqlx::query_as::<_, PassportCredentialRow>(
        r#"
        update passport_credentials
        set issue_status = 'revoked', revoked_at = now(), updated_at = now()
        where id = (
            select id
            from passport_credentials
            where user_id = $1 and issue_status = 'issued'
            order by issued_at desc nulls last, created_at desc
            limit 1
        )
        returning id,
                  user_id,
                  readiness_level,
                  verification_tier,
                  issue_status,
                  evidence_root,
                  evidence_event_count,
                  starknet_anchor_status,
                  schema_version,
                  issuer,
                  credential_hash,
                  issued_at,
                  revoked_at,
                  created_at,
                  updated_at
        "#,
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("no issued Passport credential to revoke".to_string()))?;

    Ok(Json(PassportCredentialResponse::from(row)))
}

async fn latest_passport_credential(
    state: &AppState,
    user_id: Uuid,
) -> Result<Option<PassportCredentialRow>, ApiError> {
    let row = sqlx::query_as::<_, PassportCredentialRow>(
        r#"
        select id,
               user_id,
               readiness_level,
               verification_tier,
               issue_status,
               evidence_root,
               evidence_event_count,
               starknet_anchor_status,
               schema_version,
               issuer,
               credential_hash,
               issued_at,
               revoked_at,
               created_at,
               updated_at
        from passport_credentials
        where user_id = $1
        order by created_at desc
        limit 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?;

    Ok(row)
}

async fn load_passport_eligibility(
    state: &AppState,
    user_id: Uuid,
) -> Result<PassportEligibilityResponse, ApiError> {
    let mut levels = Vec::new();

    for level in ["beginner", "intermediate", "advanced"] {
        levels.push(load_level_eligibility(state, user_id, level).await?);
    }

    let highest_eligible_level = levels
        .iter()
        .filter(|level| level.eligible)
        .max_by_key(|level| readiness_rank(&level.level))
        .map(|level| level.level.clone());

    let evidence_root = levels.iter().find_map(|level| level.evidence_root.clone());
    let evidence_event_count = levels
        .iter()
        .map(|level| level.evidence_event_count)
        .max()
        .unwrap_or(0);

    Ok(PassportEligibilityResponse {
        user_id,
        eligible: highest_eligible_level.is_some(),
        highest_eligible_level,
        evidence_root,
        evidence_event_count,
        levels,
        note: "Eligibility is derived from system proof records only. No manual proof claims are accepted.",
    })
}

async fn load_level_eligibility(
    state: &AppState,
    user_id: Uuid,
    level: &str,
) -> Result<LevelEligibilityResponse, ApiError> {
    let row = sqlx::query_as::<_, LevelEvidenceRow>(
        r#"
        select
          (select count(*) from lesson_progress where user_id = $1 and level = $2 and status = 'completed') as lesson_completed_count,
          (select count(*) from checkpoint_results where user_id = $1 and track = 'core' and level = $2 and passed = true) as core_checkpoint_passed_count,
          (select count(*) from checkpoint_results where user_id = $1 and track = 'lab' and level = $2 and passed = true) as lab_checkpoint_passed_count,
          (select count(*) from exam_attempts where user_id = $1 and track = 'core' and level = $2 and passed = true) as core_exam_passed_count,
          (select count(*) from exam_attempts where user_id = $1 and track = 'lab' and level = $2 and passed = true) as lab_exam_passed_count,
          (select count(*) from lab_attempts where user_id = $1 and level = $2 and (status = 'passed' or coalesce(score, 0) >= 70)) as lab_attempt_passed_count,
          (select count(*) from lab_attempts where user_id = $1 and level = $2 and coalesce(safety_score, 0) >= 70) as safety_passed_count,
          (select count(*) from proof_events where user_id = $1 and event_hash is not null) as evidence_event_count,
          (select event_hash from proof_events where user_id = $1 and event_hash is not null order by created_at desc, id desc limit 1) as evidence_root
        "#,
    )
    .bind(user_id)
    .bind(level)
    .fetch_one(&state.db)
    .await?;

    let proof_of_learning = row.core_exam_passed_count > 0;
    let proof_of_practice = row.lab_exam_passed_count > 0 || row.lab_attempt_passed_count > 0;
    let proof_of_safety = row.safety_passed_count > 0;
    let proof_of_readiness = row.evidence_root.is_some() && row.evidence_event_count > 0;

    let mut missing = Vec::new();
    if !proof_of_learning {
        missing.push("core level exam pass is required".to_string());
    }
    if !proof_of_practice {
        missing.push("lab practice pass is required".to_string());
    }
    if !proof_of_safety {
        missing.push("lab safety score is required".to_string());
    }
    if !proof_of_readiness {
        missing.push("backend evidence root is required".to_string());
    }

    Ok(LevelEligibilityResponse {
        level: level.to_string(),
        eligible: proof_of_learning && proof_of_practice && proof_of_safety && proof_of_readiness,
        proof_of_learning,
        proof_of_practice,
        proof_of_safety,
        proof_of_readiness,
        counts: LevelEvidenceCounts {
            lesson_completed_count: row.lesson_completed_count,
            core_checkpoint_passed_count: row.core_checkpoint_passed_count,
            lab_checkpoint_passed_count: row.lab_checkpoint_passed_count,
            core_exam_passed_count: row.core_exam_passed_count,
            lab_exam_passed_count: row.lab_exam_passed_count,
            lab_attempt_passed_count: row.lab_attempt_passed_count,
            safety_passed_count: row.safety_passed_count,
        },
        evidence_root: row.evidence_root,
        evidence_event_count: row.evidence_event_count,
        missing,
    })
}

fn readiness_rank(level: &str) -> i32 {
    match level {
        "beginner" => 1,
        "intermediate" => 2,
        "advanced" => 3,
        _ => 0,
    }
}

fn credential_hash(
    user_id: Uuid,
    readiness_level: &str,
    evidence_root: &str,
    evidence_event_count: i64,
) -> String {
    let material = format!(
        "{}|{}|{}|{}|{}|{}",
        PASSPORT_SCHEMA_VERSION,
        PASSPORT_ISSUER,
        user_id,
        readiness_level,
        evidence_root,
        evidence_event_count
    );

    let mut hasher = Sha256::new();
    hasher.update(material.as_bytes());
    let digest = hasher.finalize();

    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

impl From<PassportCredentialRow> for PassportCredentialResponse {
    fn from(row: PassportCredentialRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            readiness_level: row.readiness_level,
            verification_tier: row.verification_tier,
            issue_status: row.issue_status,
            evidence_root: row.evidence_root,
            evidence_event_count: row.evidence_event_count,
            starknet_anchor_status: row.starknet_anchor_status,
            schema_version: row.schema_version,
            issuer: row.issuer,
            credential_hash: row.credential_hash,
            issued_at: row.issued_at,
            revoked_at: row.revoked_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
            nft_ready: true,
            on_chain: false,
        }
    }
}
