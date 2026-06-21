use axum::http::HeaderMap;
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

pub async fn check_public_throttle(
    state: &AppState,
    headers: &HeaderMap,
    scope: &str,
    subject: Option<&str>,
    limit: i64,
    window_seconds: i64,
) -> Result<(), ApiError> {
    let subject_hash = public_subject_hash(headers, subject);
    let window_seconds = window_seconds.clamp(60, 86_400);
    let limit = limit.clamp(1, 10_000);

    let recent_allowed_count = sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)::bigint
        from admin_public_rate_limit_events
        where scope = $1
          and subject_hash = $2
          and allowed = true
          and occurred_at > now() - ($3::int * interval '1 second')
        "#,
    )
    .bind(scope)
    .bind(&subject_hash)
    .bind(window_seconds as i32)
    .fetch_one(&state.db)
    .await?;

    let allowed = recent_allowed_count < limit;

    sqlx::query(
        r#"
        insert into admin_public_rate_limit_events (
          id, scope, subject_hash, allowed, metadata
        ) values ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(scope)
    .bind(&subject_hash)
    .bind(allowed)
    .bind(json!({
        "limit": limit,
        "window_seconds": window_seconds,
        "recent_allowed_count": recent_allowed_count,
        "user_agent_present": header_value(headers, "user-agent").is_some(),
        "forwarded_for_present": header_value(headers, "x-forwarded-for").is_some(),
        "real_ip_present": header_value(headers, "x-real-ip").is_some()
    }))
    .execute(&state.db)
    .await?;

    if allowed {
        Ok(())
    } else {
        tracing::warn!(scope, subject_hash, limit, window_seconds, "admin public surface throttled");
        Err(ApiError::RateLimited("too many attempts; please wait before trying again".to_string()))
    }
}

fn public_subject_hash(headers: &HeaderMap, subject: Option<&str>) -> String {
    let forwarded_for = header_value(headers, "x-forwarded-for").unwrap_or_default();
    let real_ip = header_value(headers, "x-real-ip").unwrap_or_default();
    let user_agent = header_value(headers, "user-agent").unwrap_or_default();
    let subject = subject.unwrap_or("").trim().to_ascii_lowercase();

    let mut hasher = Sha256::new();
    hasher.update(forwarded_for.as_bytes());
    hasher.update(b"|");
    hasher.update(real_ip.as_bytes());
    hasher.update(b"|");
    hasher.update(user_agent.as_bytes());
    hasher.update(b"|");
    hasher.update(subject.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(256).collect())
}
