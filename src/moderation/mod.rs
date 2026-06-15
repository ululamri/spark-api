use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

#[derive(Debug, Clone)]
pub struct ModerationOutcome {
    pub decision: &'static str,
    pub status: &'static str,
    pub categories: Vec<String>,
    pub severity: &'static str,
    pub score: Option<f32>,
    pub source: &'static str,
    pub user_message: String,
    pub admin_summary: String,
}

#[derive(Debug, FromRow)]
struct RestrictionRow {
    level: String,
    reason: String,
}

#[derive(Debug, FromRow)]
struct CategoryPointsRow {
    key: String,
    strike_points: i32,
}

impl ModerationOutcome {
    pub fn allow() -> Self {
        Self {
            decision: "allow",
            status: "allowed",
            categories: Vec::new(),
            severity: "low",
            score: Some(0.0),
            source: "rules",
            user_message: String::new(),
            admin_summary: String::new(),
        }
    }

    pub fn is_allow(&self) -> bool { self.decision == "allow" }
    pub fn is_block(&self) -> bool { self.decision == "block" }
    pub fn is_review(&self) -> bool { self.decision == "review" }
}

pub async fn enforce_rate_limit(state: &AppState, user_id: Uuid, scope: &str) -> Result<(), ApiError> {
    let multiplier = restriction_multiplier(state, user_id, scope).await?;
    for window in rate_limit_windows(scope) {
        let effective_limit = ((window.limit as f32) * multiplier).floor().max(1.0) as i32;
        let used = consume_rate_window(state, user_id, scope, window.seconds, effective_limit).await?;
        if used > effective_limit {
            record_rate_limit_event(state, user_id, scope, effective_limit, used, window.seconds).await?;
            return Err(ApiError::RateLimited(format!("too many {scope} actions; please slow down and try again later")));
        }
    }
    Ok(())
}

pub fn evaluate_text(input: &str) -> ModerationOutcome {
    let text = input.to_lowercase();
    let mut categories = Vec::<String>::new();
    let mut decision = "allow";
    let mut severity = "low";
    let mut score = 0.0_f32;

    if contains_any(&text, &["csam", "minor exploitation", "child exploitation"]) {
        push_category(&mut categories, "sexual_minors");
        decision = "block";
        severity = "critical";
        score = score.max(0.99);
    }

    if contains_any(&text, &["nsfw", "adult-content", "explicit-image", "unsafe-image", "onlyfans"]) {
        push_category(&mut categories, "nsfw_explicit");
        decision = max_decision(decision, "block");
        severity = max_severity(severity, "high");
        score = score.max(0.88);
    }

    if contains_any(&text, &["slot", "slot gacor", "casino", "judi", "togel", "betting", "taruhan", "parlay", "jackpot", "maxwin", "scatter", "situs judi", "bandar"]) {
        push_category(&mut categories, "gambling");
        decision = max_decision(decision, "block");
        severity = max_severity(severity, "high");
        score = score.max(0.9);
    }

    if looks_like_private_key_request(&text) {
        push_category(&mut categories, "private_key_request");
        decision = max_decision(decision, "block");
        severity = max_severity(severity, "critical");
        score = score.max(0.96);
    }

    if contains_any(&text, &["wallet drainer", "drain wallet", "fake airdrop", "connect wallet to claim", "verifikasi seed phrase", "masukkan seed phrase", "kirim private key"]) {
        push_category(&mut categories, "financial_scam");
        decision = max_decision(decision, "block");
        severity = max_severity(severity, "critical");
        score = score.max(0.94);
    }

    if contains_any(&text, &["stealer", "keylogger", "malware", "ransomware", "credential theft", "phishing kit"]) && !is_security_education_context(&text) {
        push_category(&mut categories, "malware");
        decision = max_decision(decision, "block");
        severity = max_severity(severity, "critical");
        score = score.max(0.93);
    }

    if contains_any(&text, &["kill yourself", "i will kill you", "credible threat", "ancaman bunuh"]) {
        push_category(&mut categories, "violence_threat");
        decision = max_decision(decision, "block");
        severity = max_severity(severity, "critical");
        score = score.max(0.92);
    }

    if contains_any(&text, &["dox", "doxxing", "alamat rumahnya", "nomor ktp", "nik dia", "sebar alamat", "leak data pribadi"]) {
        push_category(&mut categories, "doxxing");
        decision = max_decision(decision, "block");
        severity = max_severity(severity, "critical");
        score = score.max(0.9);
    }

    if looks_like_trading_solicitation(&text) {
        push_category(&mut categories, "trading_solicitation");
        decision = max_decision(decision, "review");
        severity = max_severity(severity, "high");
        score = score.max(0.78);
    }

    if contains_any(&text, &["join grup vip", "join group vip", "referral exchange", "kode referral", "daftar pakai kode", "promo terbatas", "dm for paid promo", "paid promote", "endorse", "iklan murah", "wa.me/", "t.me/", "bit.ly/", "tinyurl.com/"]) {
        push_category(&mut categories, "advertising");
        decision = max_decision(decision, "review");
        severity = max_severity(severity, "medium");
        score = score.max(0.65);
    }

    if contains_any(&text, &["http://", "https://", "www."]) && contains_any(&text, &["claim", "airdrop", "bonus", "profit", "slot", "casino", "referral"]) {
        push_category(&mut categories, "external_link_risk");
        decision = max_decision(decision, "review");
        severity = max_severity(severity, "medium");
        score = score.max(0.6);
    }

    if contains_any(&text, &["bodoh", "tolol", "goblok", "idiot", "stupid", "anjing lu", "bangsat"]) {
        push_category(&mut categories, "toxicity_low");
        decision = max_decision(decision, "review");
        severity = max_severity(severity, "low");
        score = score.max(0.55);
    }

    if decision == "allow" { return ModerationOutcome::allow(); }

    ModerationOutcome {
        decision,
        status: if decision == "block" { "blocked" } else { "pending_review" },
        categories,
        severity,
        score: Some(score),
        source: "rules",
        user_message: user_message_for(decision),
        admin_summary: format!("Rule-based moderation matched {decision} categories."),
    }
}

pub fn evaluate_media_metadata(original_file_name: &str, mime_type: &str, purpose: &str, metadata: &serde_json::Value) -> ModerationOutcome {
    let text = format!("{} {} {} {}", original_file_name, mime_type, purpose, metadata).to_lowercase();
    let mut outcome = evaluate_text(&text);
    if outcome.is_allow() && mime_type.starts_with("image/") && contains_any(&text, &["nsfw", "adult-content", "explicit-image", "unsafe-image"]) {
        outcome = ModerationOutcome {
            decision: "block",
            status: "blocked",
            categories: vec!["nsfw_explicit".to_string()],
            severity: "high",
            score: Some(0.88),
            source: "rules",
            user_message: user_message_for("block"),
            admin_summary: "Risky image metadata matched unsafe-image indicators.".to_string(),
        };
    }
    outcome
}

pub async fn record_content_decision(
    state: &AppState,
    actor_user_id: Option<Uuid>,
    target_type: &str,
    target_id: Option<Uuid>,
    target_owner_user_id: Option<Uuid>,
    event_type: &str,
    outcome: &ModerationOutcome,
) -> Result<Option<Uuid>, ApiError> {
    if outcome.is_allow() && outcome.categories.is_empty() { return Ok(None); }
    let event_id = Uuid::new_v4();
    let score = outcome.score.map(|value| format!("{value:.5}"));
    sqlx::query(
        r#"
        insert into moderation_events (
          id, actor_user_id, target_type, target_id, target_owner_user_id,
          event_type, decision, categories, severity, score, source,
          user_message, admin_summary, metadata
        ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::numeric, $11, $12, $13, $14)
        "#,
    )
    .bind(event_id)
    .bind(actor_user_id)
    .bind(target_type)
    .bind(target_id)
    .bind(target_owner_user_id)
    .bind(event_type)
    .bind(outcome.decision)
    .bind(outcome.categories.clone())
    .bind(outcome.severity)
    .bind(score)
    .bind(outcome.source)
    .bind(&outcome.user_message)
    .bind(&outcome.admin_summary)
    .bind(json!({"engine": "rules", "status": outcome.status}))
    .execute(&state.db)
    .await?;

    if let Some(owner_user_id) = target_owner_user_id {
        apply_strikes(state, owner_user_id, event_id, outcome).await?;
    }
    Ok(Some(event_id))
}

async fn apply_strikes(state: &AppState, user_id: Uuid, event_id: Uuid, outcome: &ModerationOutcome) -> Result<(), ApiError> {
    if outcome.is_allow() || outcome.categories.is_empty() { return Ok(()); }
    let rows = sqlx::query_as::<_, CategoryPointsRow>("select key, strike_points from content_policy_categories where key = any($1)")
        .bind(outcome.categories.clone())
        .fetch_all(&state.db)
        .await?;
    let total_points: i32 = rows.iter().map(|row| row.strike_points).sum();
    if total_points <= 0 { return Ok(()); }
    let category_label = if rows.is_empty() { outcome.categories.join(",") } else { rows.iter().map(|row| row.key.as_str()).collect::<Vec<_>>().join(",") };

    sqlx::query(
        r#"
        insert into moderation_strikes (
          id, user_id, moderation_event_id, category, points, reason, source, decays_at, expires_at, metadata
        ) values ($1, $2, $3, $4, $5, $6, $7, now() + interval '24 hours', now() + interval '30 days', $8)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(event_id)
    .bind(category_label)
    .bind(total_points)
    .bind(&outcome.user_message)
    .bind(outcome.source)
    .bind(json!({"categories": outcome.categories.clone(), "decision": outcome.decision}))
    .execute(&state.db)
    .await?;

    let safety_level = safety_level_for(total_points);
    let restriction_level = restriction_level_for(total_points);
    let active_points = sqlx::query_scalar::<_, i32>(
        r#"
        insert into user_safety_scores (
          user_id, active_points, lifetime_points, safety_level, restriction_level,
          last_violation_at, next_decay_at
        ) values ($1, $2, $2, $3, $4, now(), now() + interval '24 hours')
        on conflict (user_id) do update set
          active_points = user_safety_scores.active_points + excluded.active_points,
          lifetime_points = user_safety_scores.lifetime_points + excluded.lifetime_points,
          safety_level = case
            when user_safety_scores.active_points + excluded.active_points >= 15 then 'admin_escalation'
            when user_safety_scores.active_points + excluded.active_points >= 10 then 'temporary_restriction'
            when user_safety_scores.active_points + excluded.active_points >= 6 then 'probation'
            when user_safety_scores.active_points + excluded.active_points >= 3 then 'reduced_velocity'
            when user_safety_scores.active_points + excluded.active_points >= 1 then 'gentle_friction'
            else 'normal'
          end,
          restriction_level = case
            when user_safety_scores.active_points + excluded.active_points >= 15 then 'hard_block'
            when user_safety_scores.active_points + excluded.active_points >= 10 then 'temporary_restriction'
            when user_safety_scores.active_points + excluded.active_points >= 6 then 'probation'
            when user_safety_scores.active_points + excluded.active_points >= 3 then 'reduced_velocity'
            when user_safety_scores.active_points + excluded.active_points >= 1 then 'gentle_friction'
            else 'none'
          end,
          last_violation_at = now(),
          next_decay_at = coalesce(user_safety_scores.next_decay_at, now() + interval '24 hours'),
          updated_at = now()
        returning active_points
        "#,
    )
    .bind(user_id)
    .bind(total_points)
    .bind(safety_level)
    .bind(restriction_level)
    .fetch_one(&state.db)
    .await?;

    if let Some((level, hours)) = restriction_for_points(active_points) {
        sqlx::query(
            r#"
            insert into user_restrictions (
              id, user_id, scope, level, reason, source_event_id, starts_at, expires_at, metadata
            ) values ($1, $2, 'all', $3, $4, $5, now(), now() + ($6::text || ' hours')::interval, $7)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(level)
        .bind(&outcome.user_message)
        .bind(event_id)
        .bind(hours.to_string())
        .bind(json!({"active_points": active_points, "source": "strike_ladder"}))
        .execute(&state.db)
        .await?;
    }
    Ok(())
}

async fn restriction_multiplier(state: &AppState, user_id: Uuid, scope: &str) -> Result<f32, ApiError> {
    let rows = sqlx::query_as::<_, RestrictionRow>(
        r#"
        select level, reason
        from user_restrictions
        where user_id = $1
          and lifted_at is null
          and (scope = $2 or scope = 'all')
          and (expires_at is null or expires_at > now())
        order by case level
          when 'hard_block' then 5
          when 'temporary_restriction' then 4
          when 'probation' then 3
          when 'reduced_velocity' then 2
          when 'gentle_friction' then 1
          else 0
        end desc
        limit 1
        "#,
    )
    .bind(user_id)
    .bind(scope)
    .fetch_all(&state.db)
    .await?;
    let Some(row) = rows.first() else { return Ok(1.0); };
    match row.level.as_str() {
        "hard_block" | "temporary_restriction" => Err(ApiError::RateLimited(if row.reason.is_empty() { "this action is temporarily restricted".to_string() } else { row.reason.clone() })),
        "probation" => Ok(0.25),
        "reduced_velocity" => Ok(0.5),
        "gentle_friction" => Ok(0.75),
        _ => Ok(1.0),
    }
}

#[derive(Clone, Copy)]
struct RateWindow { seconds: i32, limit: i32 }

fn rate_limit_windows(scope: &str) -> Vec<RateWindow> {
    match scope {
        "social_post_create" => vec![RateWindow { seconds: 3600, limit: 3 }, RateWindow { seconds: 86_400, limit: 10 }],
        "social_comment_create" => vec![RateWindow { seconds: 3600, limit: 20 }, RateWindow { seconds: 86_400, limit: 60 }],
        "social_reaction_create" => vec![RateWindow { seconds: 3600, limit: 100 }, RateWindow { seconds: 86_400, limit: 500 }],
        "social_report_create" => vec![RateWindow { seconds: 86_400, limit: 10 }],
        "media_upload" => vec![RateWindow { seconds: 86_400, limit: 20 }],
        "avatar_upload" => vec![RateWindow { seconds: 86_400, limit: 5 }],
        "profile_update" => vec![RateWindow { seconds: 86_400, limit: 10 }],
        "follow_user" => vec![RateWindow { seconds: 86_400, limit: 30 }],
        "ai_user_chat" => vec![RateWindow { seconds: 86_400, limit: 30 }],
        _ => vec![RateWindow { seconds: 3600, limit: 60 }],
    }
}

async fn consume_rate_window(state: &AppState, user_id: Uuid, scope: &str, window_seconds: i32, limit_count: i32) -> Result<i32, ApiError> {
    let now = Utc::now();
    let aligned = now.timestamp() - now.timestamp().rem_euclid(window_seconds as i64);
    let window_start: DateTime<Utc> = DateTime::from_timestamp(aligned, 0).unwrap_or(now);
    let reset_at = window_start + Duration::seconds(window_seconds as i64);
    let used = sqlx::query_scalar::<_, i32>(
        r#"
        insert into user_rate_limit_buckets (
          user_id, scope, window_start, window_seconds, used_count, limit_count, reset_at
        ) values ($1, $2, $3, $4, 1, $5, $6)
        on conflict (user_id, scope, window_start) do update set
          used_count = user_rate_limit_buckets.used_count + 1,
          limit_count = excluded.limit_count,
          reset_at = excluded.reset_at,
          updated_at = now()
        returning used_count
        "#,
    )
    .bind(user_id)
    .bind(scope)
    .bind(window_start)
    .bind(window_seconds)
    .bind(limit_count)
    .bind(reset_at)
    .fetch_one(&state.db)
    .await?;
    Ok(used)
}

async fn record_rate_limit_event(state: &AppState, user_id: Uuid, scope: &str, limit: i32, used: i32, window_seconds: i32) -> Result<(), ApiError> {
    let outcome = ModerationOutcome {
        decision: "restrict",
        status: "pending_review",
        categories: vec!["rate_limit".to_string()],
        severity: "low",
        score: Some(1.0),
        source: "system",
        user_message: "Please slow down before trying this action again.".to_string(),
        admin_summary: format!("Rate limit exceeded for {scope}: {used}/{limit} in {window_seconds}s."),
    };
    record_content_decision(state, Some(user_id), "user", Some(user_id), Some(user_id), "rate_limit", &outcome).await?;
    Ok(())
}

fn restriction_for_points(points: i32) -> Option<(&'static str, i32)> {
    if points >= 15 { Some(("hard_block", 168)) } else if points >= 10 { Some(("temporary_restriction", 168)) } else if points >= 6 { Some(("probation", 72)) } else if points >= 3 { Some(("reduced_velocity", 24)) } else if points >= 1 { Some(("gentle_friction", 1)) } else { None }
}

fn safety_level_for(points: i32) -> &'static str {
    if points >= 15 { "admin_escalation" } else if points >= 10 { "temporary_restriction" } else if points >= 6 { "probation" } else if points >= 3 { "reduced_velocity" } else if points >= 1 { "gentle_friction" } else { "normal" }
}

fn restriction_level_for(points: i32) -> &'static str {
    if points >= 15 { "hard_block" } else if points >= 10 { "temporary_restriction" } else if points >= 6 { "probation" } else if points >= 3 { "reduced_velocity" } else if points >= 1 { "gentle_friction" } else { "none" }
}

fn contains_any(text: &str, needles: &[&str]) -> bool { needles.iter().any(|needle| text.contains(needle)) }

fn push_category(categories: &mut Vec<String>, category: &str) {
    if !categories.iter().any(|item| item == category) { categories.push(category.to_string()); }
}

fn max_decision(current: &'static str, next: &'static str) -> &'static str {
    if decision_rank(next) > decision_rank(current) { next } else { current }
}

fn decision_rank(value: &str) -> i32 {
    match value { "allow" => 0, "review" => 1, "restrict" => 2, "block" => 3, _ => 0 }
}

fn max_severity(current: &'static str, next: &'static str) -> &'static str {
    if severity_rank(next) > severity_rank(current) { next } else { current }
}

fn severity_rank(value: &str) -> i32 {
    match value { "low" => 0, "medium" => 1, "high" => 2, "critical" => 3, _ => 0 }
}

fn user_message_for(decision: &str) -> String {
    match decision {
        "block" => "Content could not be published because it appears to violate the community safety policy.".to_string(),
        "review" => "Content was saved for review before it can appear publicly.".to_string(),
        "restrict" => "This action is temporarily limited for safety reasons.".to_string(),
        _ => String::new(),
    }
}

fn looks_like_private_key_request(text: &str) -> bool {
    contains_any(text, &["private key", "seed phrase", "recovery phrase", "mnemonic", "frasa pemulihan"])
        && contains_any(text, &["kirim", "send", "share", "masukkan", "input", "dm", "verifikasi", "submit"])
}

fn looks_like_trading_solicitation(text: &str) -> bool {
    contains_any(text, &["sinyal trading", "signal trading", "trading signal", "pump group", "grup pump", "entry sekarang", "entry now", "profit guarantee", "profit guaranteed", "jaminan profit", "cuan harian", "roi harian", "join vip", "grup vip", "copy trade", "copytrade"])
        || (contains_any(text, &["buy now", "beli sekarang", "long sekarang", "short sekarang"]) && contains_any(text, &["profit", "cuan", "moon", "pump", "x100", "x10"]))
}

fn is_security_education_context(text: &str) -> bool {
    contains_any(text, &["belajar", "edukasi", "pencegahan", "awareness", "mencegah", "contoh serangan", "defensive", "keamanan wallet"])
}
