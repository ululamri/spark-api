use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    auth::session::{require_current_user, CurrentUser},
    error::ApiError,
    moderation,
    state::AppState,
};

#[derive(Serialize)]
struct ScopeResponse {
    module: &'static str,
    phase: &'static str,
    implemented_now: Vec<&'static str>,
    next_backend_steps: Vec<&'static str>,
}

#[derive(Debug, Deserialize)]
struct FeedParams {
    limit: Option<i64>,
    cursor: Option<String>,
    kind: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreatePostRequest {
    kind: Option<String>,
    body: String,
    visibility: Option<String>,
    media_asset_ids: Option<Vec<Uuid>>,
    metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct CreateCommentRequest {
    body: String,
    parent_comment_id: Option<Uuid>,
    media_asset_ids: Option<Vec<Uuid>>,
    metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ReactionRequest {
    kind: String,
}

#[derive(Debug, Deserialize)]
struct ReportRequest {
    reason: String,
    details: Option<String>,
}

#[derive(Debug, Serialize)]
struct FeedResponse {
    items: Vec<HydratedPostResponse>,
    next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
struct HydratedPostResponse {
    post: SocialPostResponse,
    author: SocialProfileResponse,
    media: Vec<SocialMediaAttachment>,
    stats: SocialStatsResponse,
    viewer: SocialViewerState,
    comments: Vec<HydratedCommentResponse>,
}

#[derive(Debug, Serialize)]
struct HydratedCommentResponse {
    comment: SocialCommentResponse,
    author: SocialProfileResponse,
    media: Vec<SocialMediaAttachment>,
    stats: SocialStatsResponse,
    viewer: SocialViewerState,
}

#[derive(Debug, Serialize)]
struct SocialPostResponse {
    id: Uuid,
    author_user_id: Uuid,
    kind: String,
    body: String,
    visibility: String,
    status: String,
    published_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct SocialCommentResponse {
    id: Uuid,
    post_id: Uuid,
    author_user_id: Uuid,
    parent_comment_id: Option<Uuid>,
    body: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
struct SocialProfileResponse {
    user_id: Uuid,
    display_name: String,
    handle: Option<String>,
    bio: String,
    location: String,
    visibility: String,
    avatar_preset: String,
    avatar_url: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
struct SocialMediaAttachment {
    id: Uuid,
    original_file_name: String,
    mime_type: String,
    size_bytes: i64,
    public_url: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct SocialStatsResponse {
    comments: i64,
    reactions: Value,
}

#[derive(Debug, Serialize)]
struct SocialViewerState {
    has_reacted: bool,
    reaction_kinds: Value,
    is_following_author: bool,
    is_hidden: bool,
}

#[derive(Debug, Serialize)]
struct ActionResponse {
    ok: bool,
}

#[derive(Debug, Serialize, FromRow)]
struct ReportResponse {
    id: Uuid,
    target_type: String,
    target_id: Uuid,
    reason: String,
    details: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct SocialPostRow {
    id: Uuid,
    author_user_id: Uuid,
    kind: String,
    body: String,
    visibility: String,
    status: String,
    published_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    author_display_name: String,
    author_handle: Option<String>,
    author_bio: String,
    author_location: String,
    author_visibility: String,
    author_avatar_preset: String,
    author_avatar_url: Option<String>,
    comments_count: i64,
    reactions: Value,
    viewer_reaction_kinds: Value,
    viewer_is_following_author: bool,
    viewer_is_hidden: bool,
}

#[derive(Debug, FromRow)]
struct SocialCommentRow {
    id: Uuid,
    post_id: Uuid,
    author_user_id: Uuid,
    parent_comment_id: Option<Uuid>,
    body: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    author_display_name: String,
    author_handle: Option<String>,
    author_bio: String,
    author_location: String,
    author_visibility: String,
    author_avatar_preset: String,
    author_avatar_url: Option<String>,
    reactions: Value,
    viewer_reaction_kinds: Value,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/scope", get(scope))
        .route("/feed", get(feed))
        .route("/posts", post(create_post))
        .route("/posts/:post_id", get(get_post))
        .route("/posts/:post_id/hide", post(hide_post))
        .route("/posts/:post_id/comments", post(create_comment))
        .route("/posts/:post_id/reactions", post(upsert_post_reaction))
        .route("/posts/:post_id/reactions/:kind", delete(delete_post_reaction))
        .route("/comments/:comment_id/reactions", post(upsert_comment_reaction))
        .route("/comments/:comment_id/reactions/:kind", delete(delete_comment_reaction))
        .route("/posts/:post_id/report", post(report_post))
        .route("/comments/:comment_id/report", post(report_comment))
        .route("/profiles/:user_id", get(get_social_profile))
        .route("/profiles/:user_id/follow", post(follow_profile).delete(unfollow_profile))
}

async fn scope() -> Json<ScopeResponse> {
    Json(ScopeResponse {
        module: module_path!(),
        phase: "public-social-policy-runtime-audit-fix",
        implemented_now: vec![
            "api-backed-feed-read",
            "authenticated-post-create",
            "authenticated-comment-create",
            "feed-comment-hydration",
            "post-and-comment-reactions",
            "viewer-hide-state",
            "profile-follow-state",
            "report-queue-write",
            "media-link-attachment-hydration",
            "rule-based-content-moderation",
            "adaptive-user-rate-limits",
            "strike-and-restriction-foundation",
        ],
        next_backend_steps: vec![
            "connect admin moderation UI to policy queues",
            "add local AI moderation provider",
            "add server-side media scanner adapter",
        ],
    })
}

async fn feed(State(state): State<AppState>, headers: HeaderMap, Query(params): Query<FeedParams>) -> Result<Json<FeedResponse>, ApiError> {
    let viewer = optional_current_user(&state, &headers).await?;
    let viewer_id = viewer.as_ref().map(|user| user.id);
    let limit = params.limit.unwrap_or(20).clamp(1, 50);
    let cursor = parse_cursor(params.cursor)?;
    let kind = params.kind.as_deref().map(normalize_post_kind).transpose()?;
    let rows = sqlx::query_as::<_, SocialPostRow>(POST_FEED_SQL)
        .bind(limit)
        .bind(viewer_id)
        .bind(cursor)
        .bind(kind.as_deref())
        .fetch_all(&state.db)
        .await?;
    let next_cursor = rows.last().map(|row| row.published_at.to_rfc3339());
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        items.push(hydrate_post(&state, viewer_id, row, true).await?);
    }
    Ok(Json(FeedResponse { items, next_cursor }))
}

async fn get_post(Path(post_id): Path<Uuid>, State(state): State<AppState>, headers: HeaderMap) -> Result<Json<HydratedPostResponse>, ApiError> {
    let viewer = optional_current_user(&state, &headers).await?;
    let viewer_id = viewer.as_ref().map(|user| user.id);
    let sql = post_by_id_sql(false);
    let row = sqlx::query_as::<_, SocialPostRow>(&sql)
        .bind(post_id)
        .bind(viewer_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::BadRequest("social post not found".to_string()))?;
    Ok(Json(hydrate_post(&state, viewer_id, row, true).await?))
}

async fn create_post(State(state): State<AppState>, headers: HeaderMap, Json(payload): Json<CreatePostRequest>) -> Result<(StatusCode, Json<HydratedPostResponse>), ApiError> {
    let user = require_current_user(&state, &headers).await?;
    moderation::enforce_rate_limit(&state, user.id, "social_post_create").await?;
    ensure_profile(&state, user.id).await?;
    let kind = payload.kind.as_deref().map(normalize_post_kind).transpose()?.unwrap_or_else(|| "post".to_string());
    let visibility = payload.visibility.as_deref().map(normalize_post_visibility).transpose()?.unwrap_or_else(|| "community".to_string());
    let body = clean_body(&payload.body, "body", 4000)?;
    let media_asset_ids = payload.media_asset_ids.unwrap_or_default();
    if body.is_empty() && media_asset_ids.is_empty() {
        return Err(ApiError::BadRequest("post body or media attachment is required".to_string()));
    }

    let post_id = Uuid::new_v4();
    let outcome = moderation::evaluate_text(&body);
    if outcome.is_block() {
        moderation::record_content_decision(&state, Some(user.id), "post", Some(post_id), Some(user.id), "pre_publish_scan", &outcome).await?;
        return Err(ApiError::BadRequest(outcome.user_message));
    }

    let status = if outcome.is_review() { "hidden" } else { "published" };
    let score = outcome.score.map(|value| format!("{value:.5}"));
    sqlx::query(
        r#"
        insert into social_posts (
          id, author_user_id, kind, body, visibility, status, metadata,
          moderation_status, moderation_decision, moderation_categories,
          moderation_score, moderation_checked_at, moderation_source, moderation_message
        ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::numeric, now(), $12, $13)
        "#,
    )
    .bind(post_id)
    .bind(user.id)
    .bind(kind)
    .bind(body)
    .bind(visibility)
    .bind(status)
    .bind(payload.metadata.unwrap_or_else(|| json!({})))
    .bind(outcome.status)
    .bind(outcome.decision)
    .bind(outcome.categories.clone())
    .bind(score)
    .bind(outcome.source)
    .bind(&outcome.user_message)
    .execute(&state.db)
    .await?;

    moderation::record_content_decision(&state, Some(user.id), "post", Some(post_id), Some(user.id), "pre_publish_scan", &outcome).await?;
    attach_media_assets(&state, user.id, "social_post", post_id, media_asset_ids).await?;
    let sql = post_by_id_sql(true);
    let row = sqlx::query_as::<_, SocialPostRow>(&sql)
        .bind(post_id)
        .bind(Some(user.id))
        .bind(user.id)
        .fetch_one(&state.db)
        .await?;
    Ok((StatusCode::CREATED, Json(hydrate_post(&state, Some(user.id), row, true).await?)))
}

async fn create_comment(Path(post_id): Path<Uuid>, State(state): State<AppState>, headers: HeaderMap, Json(payload): Json<CreateCommentRequest>) -> Result<(StatusCode, Json<HydratedCommentResponse>), ApiError> {
    let user = require_current_user(&state, &headers).await?;
    moderation::enforce_rate_limit(&state, user.id, "social_comment_create").await?;
    ensure_visible_post_exists(&state, post_id).await?;
    ensure_profile(&state, user.id).await?;
    let body = clean_body(&payload.body, "body", 2000)?;
    if body.is_empty() {
        return Err(ApiError::BadRequest("comment body is required".to_string()));
    }
    if let Some(parent_comment_id) = payload.parent_comment_id {
        ensure_comment_belongs_to_post(&state, post_id, parent_comment_id).await?;
    }

    let comment_id = Uuid::new_v4();
    let outcome = moderation::evaluate_text(&body);
    if outcome.is_block() {
        moderation::record_content_decision(&state, Some(user.id), "comment", Some(comment_id), Some(user.id), "pre_publish_scan", &outcome).await?;
        return Err(ApiError::BadRequest(outcome.user_message));
    }

    let status = if outcome.is_review() { "hidden" } else { "published" };
    let score = outcome.score.map(|value| format!("{value:.5}"));
    sqlx::query(
        r#"
        insert into social_comments (
          id, post_id, author_user_id, parent_comment_id, body, status, metadata,
          moderation_status, moderation_decision, moderation_categories,
          moderation_score, moderation_checked_at, moderation_source, moderation_message
        ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::numeric, now(), $12, $13)
        "#,
    )
    .bind(comment_id)
    .bind(post_id)
    .bind(user.id)
    .bind(payload.parent_comment_id)
    .bind(body)
    .bind(status)
    .bind(payload.metadata.unwrap_or_else(|| json!({})))
    .bind(outcome.status)
    .bind(outcome.decision)
    .bind(outcome.categories.clone())
    .bind(score)
    .bind(outcome.source)
    .bind(&outcome.user_message)
    .execute(&state.db)
    .await?;

    moderation::record_content_decision(&state, Some(user.id), "comment", Some(comment_id), Some(user.id), "pre_publish_scan", &outcome).await?;
    attach_media_assets(&state, user.id, "social_comment", comment_id, payload.media_asset_ids.unwrap_or_default()).await?;
    let sql = comment_by_id_sql(true);
    let row = sqlx::query_as::<_, SocialCommentRow>(&sql)
        .bind(comment_id)
        .bind(Some(user.id))
        .bind(user.id)
        .fetch_one(&state.db)
        .await?;
    Ok((StatusCode::CREATED, Json(hydrate_comment(&state, Some(user.id), row).await?)))
}

async fn upsert_post_reaction(Path(post_id): Path<Uuid>, State(state): State<AppState>, headers: HeaderMap, Json(payload): Json<ReactionRequest>) -> Result<Json<ActionResponse>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    moderation::enforce_rate_limit(&state, user.id, "social_reaction_create").await?;
    ensure_visible_post_exists(&state, post_id).await?;
    let kind = normalize_reaction_kind(&payload.kind)?;
    sqlx::query("insert into social_reactions (id, user_id, post_id, kind) values ($1, $2, $3, $4) on conflict (user_id, post_id, kind) where post_id is not null do update set updated_at = now()")
        .bind(Uuid::new_v4())
        .bind(user.id)
        .bind(post_id)
        .bind(kind)
        .execute(&state.db)
        .await?;
    Ok(Json(ActionResponse { ok: true }))
}

async fn delete_post_reaction(Path((post_id, kind)): Path<(Uuid, String)>, State(state): State<AppState>, headers: HeaderMap) -> Result<Json<ActionResponse>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let kind = normalize_reaction_kind(&kind)?;
    sqlx::query("delete from social_reactions where user_id = $1 and post_id = $2 and kind = $3")
        .bind(user.id)
        .bind(post_id)
        .bind(kind)
        .execute(&state.db)
        .await?;
    Ok(Json(ActionResponse { ok: true }))
}

async fn upsert_comment_reaction(Path(comment_id): Path<Uuid>, State(state): State<AppState>, headers: HeaderMap, Json(payload): Json<ReactionRequest>) -> Result<Json<ActionResponse>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    moderation::enforce_rate_limit(&state, user.id, "social_reaction_create").await?;
    ensure_visible_comment_exists(&state, comment_id).await?;
    let kind = normalize_reaction_kind(&payload.kind)?;
    sqlx::query("insert into social_reactions (id, user_id, comment_id, kind) values ($1, $2, $3, $4) on conflict (user_id, comment_id, kind) where comment_id is not null do update set updated_at = now()")
        .bind(Uuid::new_v4())
        .bind(user.id)
        .bind(comment_id)
        .bind(kind)
        .execute(&state.db)
        .await?;
    Ok(Json(ActionResponse { ok: true }))
}

async fn delete_comment_reaction(Path((comment_id, kind)): Path<(Uuid, String)>, State(state): State<AppState>, headers: HeaderMap) -> Result<Json<ActionResponse>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    let kind = normalize_reaction_kind(&kind)?;
    sqlx::query("delete from social_reactions where user_id = $1 and comment_id = $2 and kind = $3")
        .bind(user.id)
        .bind(comment_id)
        .bind(kind)
        .execute(&state.db)
        .await?;
    Ok(Json(ActionResponse { ok: true }))
}

async fn hide_post(Path(post_id): Path<Uuid>, State(state): State<AppState>, headers: HeaderMap) -> Result<Json<ActionResponse>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    ensure_visible_post_exists(&state, post_id).await?;
    sqlx::query("insert into social_post_hides (user_id, post_id, reason) values ($1, $2, 'viewer_hidden') on conflict (user_id, post_id) do nothing")
        .bind(user.id)
        .bind(post_id)
        .execute(&state.db)
        .await?;
    Ok(Json(ActionResponse { ok: true }))
}

async fn report_post(Path(post_id): Path<Uuid>, State(state): State<AppState>, headers: HeaderMap, Json(payload): Json<ReportRequest>) -> Result<(StatusCode, Json<ReportResponse>), ApiError> {
    let user = require_current_user(&state, &headers).await?;
    moderation::enforce_rate_limit(&state, user.id, "social_report_create").await?;
    ensure_visible_post_exists(&state, post_id).await?;
    let report = create_report(&state, user.id, "post", post_id, payload).await?;
    Ok((StatusCode::CREATED, Json(report)))
}

async fn report_comment(Path(comment_id): Path<Uuid>, State(state): State<AppState>, headers: HeaderMap, Json(payload): Json<ReportRequest>) -> Result<(StatusCode, Json<ReportResponse>), ApiError> {
    let user = require_current_user(&state, &headers).await?;
    moderation::enforce_rate_limit(&state, user.id, "social_report_create").await?;
    ensure_visible_comment_exists(&state, comment_id).await?;
    let report = create_report(&state, user.id, "comment", comment_id, payload).await?;
    Ok((StatusCode::CREATED, Json(report)))
}

async fn get_social_profile(Path(user_id): Path<Uuid>, State(state): State<AppState>) -> Result<Json<SocialProfileResponse>, ApiError> {
    Ok(Json(visible_profile(&state, user_id).await?))
}

async fn follow_profile(Path(user_id): Path<Uuid>, State(state): State<AppState>, headers: HeaderMap) -> Result<Json<ActionResponse>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    moderation::enforce_rate_limit(&state, user.id, "follow_user").await?;
    if user.id == user_id {
        return Err(ApiError::BadRequest("cannot follow yourself".to_string()));
    }
    visible_profile(&state, user_id).await?;
    sqlx::query("insert into social_follows (follower_user_id, followed_user_id, status) values ($1, $2, 'following') on conflict (follower_user_id, followed_user_id) do update set status = 'following', updated_at = now()")
        .bind(user.id)
        .bind(user_id)
        .execute(&state.db)
        .await?;
    Ok(Json(ActionResponse { ok: true }))
}

async fn unfollow_profile(Path(user_id): Path<Uuid>, State(state): State<AppState>, headers: HeaderMap) -> Result<Json<ActionResponse>, ApiError> {
    let user = require_current_user(&state, &headers).await?;
    sqlx::query("delete from social_follows where follower_user_id = $1 and followed_user_id = $2")
        .bind(user.id)
        .bind(user_id)
        .execute(&state.db)
        .await?;
    Ok(Json(ActionResponse { ok: true }))
}

async fn optional_current_user(state: &AppState, headers: &HeaderMap) -> Result<Option<CurrentUser>, ApiError> {
    match require_current_user(state, headers).await {
        Ok(user) => Ok(Some(user)),
        Err(ApiError::Unauthorized) => Ok(None),
        Err(error) => Err(error),
    }
}

async fn hydrate_post(state: &AppState, viewer_id: Option<Uuid>, row: SocialPostRow, include_comments: bool) -> Result<HydratedPostResponse, ApiError> {
    let media = fetch_media_for_entity(state, "social_post", row.id, viewer_id).await?;
    let comments = if include_comments {
        let rows = sqlx::query_as::<_, SocialCommentRow>(COMMENTS_FOR_POST_SQL).bind(row.id).bind(viewer_id).fetch_all(&state.db).await?;
        let mut output = Vec::with_capacity(rows.len());
        for comment in rows {
            output.push(hydrate_comment(state, viewer_id, comment).await?);
        }
        output
    } else {
        Vec::new()
    };
    Ok(HydratedPostResponse {
        post: SocialPostResponse { id: row.id, author_user_id: row.author_user_id, kind: row.kind, body: row.body, visibility: row.visibility, status: row.status, published_at: row.published_at, created_at: row.created_at, updated_at: row.updated_at },
        author: SocialProfileResponse { user_id: row.author_user_id, display_name: row.author_display_name, handle: row.author_handle, bio: row.author_bio, location: row.author_location, visibility: row.author_visibility, avatar_preset: row.author_avatar_preset, avatar_url: row.author_avatar_url },
        media,
        stats: SocialStatsResponse { comments: row.comments_count, reactions: row.reactions },
        viewer: SocialViewerState { has_reacted: json_array_has_items(&row.viewer_reaction_kinds), reaction_kinds: row.viewer_reaction_kinds, is_following_author: row.viewer_is_following_author, is_hidden: row.viewer_is_hidden },
        comments,
    })
}

async fn hydrate_comment(state: &AppState, viewer_id: Option<Uuid>, row: SocialCommentRow) -> Result<HydratedCommentResponse, ApiError> {
    let media = fetch_media_for_entity(state, "social_comment", row.id, viewer_id).await?;
    Ok(HydratedCommentResponse {
        comment: SocialCommentResponse { id: row.id, post_id: row.post_id, author_user_id: row.author_user_id, parent_comment_id: row.parent_comment_id, body: row.body, status: row.status, created_at: row.created_at, updated_at: row.updated_at },
        author: SocialProfileResponse { user_id: row.author_user_id, display_name: row.author_display_name, handle: row.author_handle, bio: row.author_bio, location: row.author_location, visibility: row.author_visibility, avatar_preset: row.author_avatar_preset, avatar_url: row.author_avatar_url },
        media,
        stats: SocialStatsResponse { comments: 0, reactions: row.reactions },
        viewer: SocialViewerState { has_reacted: json_array_has_items(&row.viewer_reaction_kinds), reaction_kinds: row.viewer_reaction_kinds, is_following_author: false, is_hidden: false },
    })
}

async fn fetch_media_for_entity(state: &AppState, entity_type: &str, entity_id: Uuid, viewer_id: Option<Uuid>) -> Result<Vec<SocialMediaAttachment>, ApiError> {
    let rows = sqlx::query_as::<_, SocialMediaAttachment>(
        r#"
        select ma.id, ma.original_file_name, ma.mime_type, ma.size_bytes, ma.public_url, ma.created_at
        from media_links ml
        join media_assets ma on ma.id = ml.media_asset_id
        where ml.entity_type = $1
          and ml.entity_id = $2
          and ma.status = 'uploaded'
          and ma.moderation_status in ('allowed', 'restored')
          and (ma.visibility = 'public' or ma.owner_user_id = $3::uuid)
        order by ml.created_at asc
        "#,
    )
    .bind(entity_type)
    .bind(entity_id.to_string())
    .bind(viewer_id)
    .fetch_all(&state.db)
    .await?;
    Ok(rows)
}

async fn attach_media_assets(state: &AppState, owner_user_id: Uuid, entity_type: &str, entity_id: Uuid, media_asset_ids: Vec<Uuid>) -> Result<(), ApiError> {
    for asset_id in media_asset_ids {
        let inserted = sqlx::query_scalar::<_, Uuid>(
            r#"
            insert into media_links (media_asset_id, entity_type, entity_id, purpose)
            select id, $2, $3, 'community'
            from media_assets
            where id = $1 and owner_user_id = $4 and status = 'uploaded' and visibility = 'public' and moderation_status in ('allowed', 'restored')
            returning media_asset_id
            "#,
        )
        .bind(asset_id)
        .bind(entity_type)
        .bind(entity_id.to_string())
        .bind(owner_user_id)
        .fetch_optional(&state.db)
        .await?;
        if inserted.is_none() {
            return Err(ApiError::BadRequest("media attachment must be an allowed uploaded public asset owned by the current user".to_string()));
        }
    }
    Ok(())
}

async fn create_report(state: &AppState, reporter_user_id: Uuid, target_type: &str, target_id: Uuid, payload: ReportRequest) -> Result<ReportResponse, ApiError> {
    let reason = normalize_report_reason(&payload.reason)?;
    let details = clean_optional_details(payload.details.as_deref())?;
    let row = sqlx::query_as::<_, ReportResponse>(
        r#"
        insert into social_reports (id, reporter_user_id, target_type, target_id, reason, details)
        values ($1, $2, $3, $4, $5, $6)
        on conflict (reporter_user_id, target_type, target_id, reason) where status = 'pending'
        do update set details = excluded.details, updated_at = now()
        returning id, target_type, target_id, reason, details, status, created_at, updated_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(reporter_user_id)
    .bind(target_type)
    .bind(target_id)
    .bind(reason)
    .bind(details)
    .fetch_one(&state.db)
    .await?;
    Ok(row)
}

async fn visible_profile(state: &AppState, user_id: Uuid) -> Result<SocialProfileResponse, ApiError> {
    let row = sqlx::query_as::<_, SocialProfileResponse>(
        r#"
        select u.id as user_id,
               coalesce(nullif(p.display_name, ''), split_part(u.email, '@', 1), 'Pengguna Spark') as display_name,
               p.handle,
               coalesce(p.bio, '') as bio,
               coalesce(p.location, '') as location,
               coalesce(p.visibility, 'community') as visibility,
               coalesce(p.avatar_preset, 'spark') as avatar_preset,
               p.avatar_url
        from users u
        left join profiles p on p.user_id = u.id
        where u.id = $1 and u.status = 'active' and coalesce(p.visibility, 'community') in ('public', 'community')
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("profile not visible".to_string()))?;
    Ok(row)
}

async fn ensure_profile(state: &AppState, user_id: Uuid) -> Result<(), ApiError> {
    sqlx::query("insert into profiles (user_id, display_name, visibility, avatar_preset) select id, coalesce(nullif(split_part(email, '@', 1), ''), 'Pengguna Spark'), 'community', 'spark' from users where id = $1 and status = 'active' on conflict (user_id) do nothing")
        .bind(user_id)
        .execute(&state.db)
        .await?;
    Ok(())
}

async fn ensure_visible_post_exists(state: &AppState, post_id: Uuid) -> Result<(), ApiError> {
    let exists = sqlx::query_scalar::<_, bool>("select exists(select 1 from social_posts where id = $1 and status = 'published' and visibility in ('public', 'community') and moderation_status in ('allowed', 'restored'))")
        .bind(post_id)
        .fetch_one(&state.db)
        .await?;
    if exists { Ok(()) } else { Err(ApiError::BadRequest("social post not found".to_string())) }
}

async fn ensure_visible_comment_exists(state: &AppState, comment_id: Uuid) -> Result<(), ApiError> {
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        select exists(
          select 1 from social_comments c
          join social_posts p on p.id = c.post_id
          where c.id = $1 and c.status = 'published' and c.moderation_status in ('allowed', 'restored')
            and p.status = 'published' and p.moderation_status in ('allowed', 'restored') and p.visibility in ('public', 'community')
        )
        "#,
    )
    .bind(comment_id)
    .fetch_one(&state.db)
    .await?;
    if exists { Ok(()) } else { Err(ApiError::BadRequest("social comment not found".to_string())) }
}

async fn ensure_comment_belongs_to_post(state: &AppState, post_id: Uuid, comment_id: Uuid) -> Result<(), ApiError> {
    let exists = sqlx::query_scalar::<_, bool>("select exists(select 1 from social_comments where id = $1 and post_id = $2 and status = 'published' and moderation_status in ('allowed', 'restored'))")
        .bind(comment_id)
        .bind(post_id)
        .fetch_one(&state.db)
        .await?;
    if exists { Ok(()) } else { Err(ApiError::BadRequest("parent comment does not belong to this post".to_string())) }
}

fn parse_cursor(input: Option<String>) -> Result<Option<DateTime<Utc>>, ApiError> {
    let Some(value) = input else { return Ok(None); };
    let value = value.trim();
    if value.is_empty() { return Ok(None); }
    DateTime::parse_from_rfc3339(value).map(|parsed| Some(parsed.with_timezone(&Utc))).map_err(|_| ApiError::BadRequest("cursor must be an RFC3339 timestamp".to_string()))
}

fn clean_body(input: &str, field: &str, max: usize) -> Result<String, ApiError> {
    let value = input.trim();
    if value.chars().count() > max { return Err(ApiError::BadRequest(format!("{field} is too long"))); }
    if value.chars().any(char::is_control) { return Err(ApiError::BadRequest(format!("{field} cannot contain control characters"))); }
    Ok(value.to_string())
}

fn clean_optional_details(input: Option<&str>) -> Result<String, ApiError> {
    input.map(|value| clean_body(value, "details", 2000)).transpose().map(|value| value.unwrap_or_default())
}

fn normalize_post_kind(input: &str) -> Result<String, ApiError> {
    let value = input.trim().to_ascii_lowercase();
    match value.as_str() {
        "post" | "reflection" | "question" | "proof" | "milestone" | "update" => Ok(value),
        _ => Err(ApiError::BadRequest("kind must be post, reflection, question, proof, milestone, or update".to_string())),
    }
}

fn normalize_post_visibility(input: &str) -> Result<String, ApiError> {
    let value = input.trim().to_ascii_lowercase();
    match value.as_str() {
        "public" | "community" => Ok(value),
        _ => Err(ApiError::BadRequest("visibility must be public or community for the public social feed".to_string())),
    }
}

fn normalize_reaction_kind(input: &str) -> Result<String, ApiError> {
    let value = input.trim().to_ascii_lowercase();
    match value.as_str() {
        "like" | "support" | "insightful" | "celebrate" => Ok(value),
        _ => Err(ApiError::BadRequest("reaction kind must be like, support, insightful, or celebrate".to_string())),
    }
}

fn normalize_report_reason(input: &str) -> Result<String, ApiError> {
    let value = input.trim().to_ascii_lowercase();
    match value.as_str() {
        "spam" | "abuse" | "harassment" | "unsafe" | "privacy" | "misleading" | "other" => Ok(value),
        _ => Err(ApiError::BadRequest("reason must be spam, abuse, harassment, unsafe, privacy, misleading, or other".to_string())),
    }
}

fn json_array_has_items(value: &Value) -> bool {
    value.as_array().map(|items| !items.is_empty()).unwrap_or(false)
}

fn post_by_id_sql(for_author: bool) -> String {
    format!(
        "{}{}",
        POST_SELECT,
        if for_author {
            "where p.id = $1 and p.author_user_id = $3"
        } else {
            "where p.id = $1 and p.status = 'published' and p.visibility in ('public', 'community') and p.moderation_status in ('allowed', 'restored')"
        }
    )
}

fn comment_by_id_sql(for_author: bool) -> String {
    format!(
        "{}{}",
        COMMENT_SELECT,
        if for_author {
            "where c.id = $1 and c.author_user_id = $3"
        } else {
            "where c.id = $1 and c.status = 'published' and c.moderation_status in ('allowed', 'restored')"
        }
    )
}

const POST_SELECT: &str = r#"
select p.id,
       p.author_user_id,
       p.kind,
       p.body,
       p.visibility,
       p.status,
       p.published_at,
       p.created_at,
       p.updated_at,
       coalesce(nullif(pr.display_name, ''), 'Pengguna Spark') as author_display_name,
       pr.handle as author_handle,
       coalesce(pr.bio, '') as author_bio,
       coalesce(pr.location, '') as author_location,
       coalesce(pr.visibility, 'community') as author_visibility,
       coalesce(pr.avatar_preset, 'spark') as author_avatar_preset,
       pr.avatar_url as author_avatar_url,
       (select count(*) from social_comments sc where sc.post_id = p.id and sc.status = 'published' and sc.moderation_status in ('allowed', 'restored')) as comments_count,
       coalesce((select jsonb_object_agg(kind, total) from (select sr.kind, count(*)::bigint as total from social_reactions sr where sr.post_id = p.id group by sr.kind) reaction_counts), '{}'::jsonb) as reactions,
       coalesce((select jsonb_agg(sr.kind order by sr.kind) from social_reactions sr where sr.post_id = p.id and sr.user_id = $2::uuid), '[]'::jsonb) as viewer_reaction_kinds,
       exists(select 1 from social_follows sf where sf.follower_user_id = $2::uuid and sf.followed_user_id = p.author_user_id and sf.status = 'following') as viewer_is_following_author,
       exists(select 1 from social_post_hides sph where sph.user_id = $2::uuid and sph.post_id = p.id) as viewer_is_hidden
from social_posts p
left join profiles pr on pr.user_id = p.author_user_id
"#;

const POST_FEED_SQL: &str = r#"
select p.id,
       p.author_user_id,
       p.kind,
       p.body,
       p.visibility,
       p.status,
       p.published_at,
       p.created_at,
       p.updated_at,
       coalesce(nullif(pr.display_name, ''), 'Pengguna Spark') as author_display_name,
       pr.handle as author_handle,
       coalesce(pr.bio, '') as author_bio,
       coalesce(pr.location, '') as author_location,
       coalesce(pr.visibility, 'community') as author_visibility,
       coalesce(pr.avatar_preset, 'spark') as author_avatar_preset,
       pr.avatar_url as author_avatar_url,
       (select count(*) from social_comments sc where sc.post_id = p.id and sc.status = 'published' and sc.moderation_status in ('allowed', 'restored')) as comments_count,
       coalesce((select jsonb_object_agg(kind, total) from (select sr.kind, count(*)::bigint as total from social_reactions sr where sr.post_id = p.id group by sr.kind) reaction_counts), '{}'::jsonb) as reactions,
       coalesce((select jsonb_agg(sr.kind order by sr.kind) from social_reactions sr where sr.post_id = p.id and sr.user_id = $2::uuid), '[]'::jsonb) as viewer_reaction_kinds,
       exists(select 1 from social_follows sf where sf.follower_user_id = $2::uuid and sf.followed_user_id = p.author_user_id and sf.status = 'following') as viewer_is_following_author,
       exists(select 1 from social_post_hides sph where sph.user_id = $2::uuid and sph.post_id = p.id) as viewer_is_hidden
from social_posts p
left join profiles pr on pr.user_id = p.author_user_id
where p.status = 'published'
  and p.visibility in ('public', 'community')
  and p.moderation_status in ('allowed', 'restored')
  and ($3::timestamptz is null or p.published_at < $3)
  and ($4::text is null or p.kind = $4)
  and not exists (select 1 from social_post_hides sph where sph.user_id = $2::uuid and sph.post_id = p.id)
order by p.published_at desc, p.id desc
limit $1
"#;

const COMMENTS_FOR_POST_SQL: &str = r#"
select c.id,
       c.post_id,
       c.author_user_id,
       c.parent_comment_id,
       c.body,
       c.status,
       c.created_at,
       c.updated_at,
       coalesce(nullif(pr.display_name, ''), 'Pengguna Spark') as author_display_name,
       pr.handle as author_handle,
       coalesce(pr.bio, '') as author_bio,
       coalesce(pr.location, '') as author_location,
       coalesce(pr.visibility, 'community') as author_visibility,
       coalesce(pr.avatar_preset, 'spark') as author_avatar_preset,
       pr.avatar_url as author_avatar_url,
       coalesce((select jsonb_object_agg(kind, total) from (select sr.kind, count(*)::bigint as total from social_reactions sr where sr.comment_id = c.id group by sr.kind) reaction_counts), '{}'::jsonb) as reactions,
       coalesce((select jsonb_agg(sr.kind order by sr.kind) from social_reactions sr where sr.comment_id = c.id and sr.user_id = $2::uuid), '[]'::jsonb) as viewer_reaction_kinds
from social_comments c
left join profiles pr on pr.user_id = c.author_user_id
join social_posts p on p.id = c.post_id and p.status = 'published' and p.moderation_status in ('allowed', 'restored')
where c.post_id = $1 and c.status = 'published' and c.moderation_status in ('allowed', 'restored')
order by c.created_at asc, c.id asc
limit 100
"#;

const COMMENT_SELECT: &str = r#"
select c.id,
       c.post_id,
       c.author_user_id,
       c.parent_comment_id,
       c.body,
       c.status,
       c.created_at,
       c.updated_at,
       coalesce(nullif(pr.display_name, ''), 'Pengguna Spark') as author_display_name,
       pr.handle as author_handle,
       coalesce(pr.bio, '') as author_bio,
       coalesce(pr.location, '') as author_location,
       coalesce(pr.visibility, 'community') as author_visibility,
       coalesce(pr.avatar_preset, 'spark') as author_avatar_preset,
       pr.avatar_url as author_avatar_url,
       coalesce((select jsonb_object_agg(kind, total) from (select sr.kind, count(*)::bigint as total from social_reactions sr where sr.comment_id = c.id group by sr.kind) reaction_counts), '{}'::jsonb) as reactions,
       coalesce((select jsonb_agg(sr.kind order by sr.kind) from social_reactions sr where sr.comment_id = c.id and sr.user_id = $2::uuid), '[]'::jsonb) as viewer_reaction_kinds
from social_comments c
left join profiles pr on pr.user_id = c.author_user_id
join social_posts p on p.id = c.post_id and p.status = 'published' and p.moderation_status in ('allowed', 'restored')
"#;
