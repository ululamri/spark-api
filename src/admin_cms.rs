use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{admin_auth, error::ApiError, state::AppState};

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

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/scope", get(scope))
        .route("/items", get(items).post(create_item))
        .route("/items/:item_id", get(item_detail))
        .route("/items/:item_id/revisions", post(create_revision))
        .route("/items/:item_id/publish", post(publish_item))
        .route("/items/:item_id/archive", post(archive_item))
}

#[derive(Serialize)]
struct ScopeData {
    module: &'static str,
    phase: &'static str,
    routes: Vec<&'static str>,
    kinds: Vec<&'static str>,
    statuses: Vec<&'static str>,
    auth_model: &'static str,
}

async fn scope(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminEnvelope<ScopeData>>, ApiError> {
    admin_auth::authorize_with_capability(&state, &headers, "content_read").await?;
    Ok(success(ScopeData {
        module: module_path!(),
        phase: "admin-cms-foundation",
        routes: vec![
            "GET /api/admin/cms/items",
            "POST /api/admin/cms/items",
            "GET /api/admin/cms/items/:item_id",
            "POST /api/admin/cms/items/:item_id/revisions",
            "POST /api/admin/cms/items/:item_id/publish",
            "POST /api/admin/cms/items/:item_id/archive",
        ],
        kinds: vec!["core_lesson", "lab"],
        statuses: vec!["draft", "review", "published", "archived"],
        auth_model: "legacy superadmin root plus delegated admin content capabilities",
    }))
}

#[derive(Debug, Deserialize)]
struct ItemsQuery {
    kind: Option<String>,
    status: Option<String>,
    q: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

impl ItemsQuery {
    fn paging(&self) -> (i64, i64) {
        (
            self.limit.unwrap_or(50).clamp(1, 100),
            self.offset.unwrap_or(0).max(0),
        )
    }
}

#[derive(Debug, Serialize, FromRow)]
struct CmsItemRow {
    id: Uuid,
    kind: String,
    slug: String,
    title: String,
    status: String,
    current_revision_id: Option<Uuid>,
    current_version: Option<i32>,
    published_at: Option<DateTime<Utc>>,
    archived_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
struct CmsRevisionRow {
    id: Uuid,
    item_id: Uuid,
    version: i32,
    payload: Value,
    summary: String,
    created_by_kind: String,
    created_by_user_id: Option<Uuid>,
    created_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct ItemsData {
    items: Vec<CmsItemRow>,
    total: i64,
    limit: i64,
    offset: i64,
    data_source: &'static str,
}

#[derive(Serialize)]
struct ItemDetailData {
    item: CmsItemRow,
    current_revision: Option<CmsRevisionRow>,
    revisions: Vec<CmsRevisionRow>,
    data_source: &'static str,
}

async fn items(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ItemsQuery>,
) -> Result<Json<AdminEnvelope<ItemsData>>, ApiError> {
    admin_auth::authorize_with_capability(&state, &headers, "content_read").await?;
    let (limit, offset) = query.paging();
    let kind = query.kind.as_deref().map(normalize_kind).transpose()?;
    let status = query.status.as_deref().map(normalize_status).transpose()?;
    let search = clean_search(query.q.as_deref())?;

    let total = sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)
        from admin_cms_items item
        where ($1::text is null or item.kind = $1)
          and ($2::text is null or item.status = $2)
          and ($3::text is null or item.slug ilike '%' || $3 || '%' or item.title ilike '%' || $3 || '%')
        "#,
    )
    .bind(kind.as_deref())
    .bind(status.as_deref())
    .bind(search.as_deref())
    .fetch_one(&state.db)
    .await?;

    let rows = sqlx::query_as::<_, CmsItemRow>(
        r#"
        select item.id,
               item.kind,
               item.slug,
               item.title,
               item.status,
               item.current_revision_id,
               rev.version as current_version,
               item.published_at,
               item.archived_at,
               item.created_at,
               item.updated_at
        from admin_cms_items item
        left join admin_cms_revisions rev on rev.id = item.current_revision_id
        where ($1::text is null or item.kind = $1)
          and ($2::text is null or item.status = $2)
          and ($3::text is null or item.slug ilike '%' || $3 || '%' or item.title ilike '%' || $3 || '%')
        order by item.updated_at desc, item.created_at desc, item.id desc
        limit $4 offset $5
        "#,
    )
    .bind(kind.as_deref())
    .bind(status.as_deref())
    .bind(search.as_deref())
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    Ok(success(ItemsData {
        items: rows,
        total,
        limit,
        offset,
        data_source: "database",
    }))
}

async fn item_detail(
    Path(item_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminEnvelope<ItemDetailData>>, ApiError> {
    admin_auth::authorize_with_capability(&state, &headers, "content_read").await?;
    let item = fetch_item(&state, item_id).await?;
    let current_revision = match item.current_revision_id {
        Some(revision_id) => fetch_revision(&state, revision_id).await?,
        None => None,
    };
    let revisions = sqlx::query_as::<_, CmsRevisionRow>(
        r#"
        select id, item_id, version, payload, summary, created_by_kind, created_by_user_id, created_at
        from admin_cms_revisions
        where item_id = $1
        order by version desc
        limit 50
        "#,
    )
    .bind(item_id)
    .fetch_all(&state.db)
    .await?;

    Ok(success(ItemDetailData {
        item,
        current_revision,
        revisions,
        data_source: "database",
    }))
}

#[derive(Debug, Deserialize)]
struct CreateItemRequest {
    kind: String,
    slug: String,
    title: String,
    payload: Option<Value>,
    summary: Option<String>,
    status: Option<String>,
    metadata: Option<Value>,
}

#[derive(Serialize)]
struct ItemWriteData {
    item: CmsItemRow,
    revision: CmsRevisionRow,
}

async fn create_item(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateItemRequest>,
) -> Result<Json<AdminEnvelope<ItemWriteData>>, ApiError> {
    let actor = admin_auth::authorize_with_capability(&state, &headers, "content_create").await?;
    let kind = normalize_kind(&payload.kind)?;
    let slug = clean_slug(&payload.slug)?;
    let title = clean_title(&payload.title)?;
    let status = payload
        .status
        .as_deref()
        .map(normalize_initial_status)
        .transpose()?
        .unwrap_or_else(|| "draft".to_string());
    let revision_payload = clean_payload(payload.payload.unwrap_or_else(|| json!({})))?;
    let summary = clean_summary(payload.summary.as_deref())?;
    let metadata = clean_metadata(payload.metadata.unwrap_or_else(|| json!({})))?;
    let item_id = Uuid::new_v4();
    let revision_id = Uuid::new_v4();

    let mut tx = state.db.begin().await?;
    sqlx::query(
        r#"
        insert into admin_cms_items (
          id, kind, slug, title, status, current_revision_id,
          created_by_kind, created_by_user_id, updated_by_kind, updated_by_user_id,
          published_at, metadata
        ) values ($1, $2, $3, $4, $5, $6, $7, $8, $7, $8,
                  case when $5 = 'published' then now() else null end, $9)
        "#,
    )
    .bind(item_id)
    .bind(&kind)
    .bind(&slug)
    .bind(&title)
    .bind(&status)
    .bind(revision_id)
    .bind(&actor.actor_kind)
    .bind(actor.actor_user_id)
    .bind(metadata)
    .execute(&mut *tx)
    .await?;

    let revision = sqlx::query_as::<_, CmsRevisionRow>(
        r#"
        insert into admin_cms_revisions (
          id, item_id, version, payload, summary, created_by_kind, created_by_user_id
        ) values ($1, $2, 1, $3, $4, $5, $6)
        returning id, item_id, version, payload, summary, created_by_kind, created_by_user_id, created_at
        "#,
    )
    .bind(revision_id)
    .bind(item_id)
    .bind(revision_payload)
    .bind(&summary)
    .bind(&actor.actor_kind)
    .bind(actor.actor_user_id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        insert into admin_cms_publish_events (
          id, item_id, revision_id, action, actor_kind, actor_user_id, reason, metadata
        ) values ($1, $2, $3, 'create', $4, $5, $6, $7)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(item_id)
    .bind(revision_id)
    .bind(&actor.actor_kind)
    .bind(actor.actor_user_id)
    .bind(&summary)
    .bind(json!({"status": status}))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    admin_auth::audit(
        &state,
        &actor,
        "cms_item_create",
        &kind,
        actor.actor_user_id,
        Some(item_id),
        &actor.capabilities,
        "CMS item was created.",
        json!({"slug": slug, "status": status, "revision_id": revision_id}),
    )
    .await?;

    let item = fetch_item(&state, item_id).await?;
    Ok(success(ItemWriteData { item, revision }))
}

#[derive(Debug, Deserialize)]
struct CreateRevisionRequest {
    payload: Value,
    summary: Option<String>,
    title: Option<String>,
    status: Option<String>,
}

async fn create_revision(
    Path(item_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateRevisionRequest>,
) -> Result<Json<AdminEnvelope<ItemWriteData>>, ApiError> {
    let actor = admin_auth::authorize_with_capability(&state, &headers, "content_edit").await?;
    let item = fetch_item(&state, item_id).await?;
    if item.status == "archived" {
        return Err(ApiError::BadRequest("archived CMS item cannot be revised".to_string()));
    }
    let revision_payload = clean_payload(payload.payload)?;
    let summary = clean_summary(payload.summary.as_deref())?;
    let new_title = payload.title.as_deref().map(clean_title).transpose()?;
    let new_status = payload.status.as_deref().map(normalize_initial_status).transpose()?;
    let revision_id = Uuid::new_v4();

    let mut tx = state.db.begin().await?;
    let next_version = sqlx::query_scalar::<_, i32>(
        "select coalesce(max(version), 0) + 1 from admin_cms_revisions where item_id = $1",
    )
    .bind(item_id)
    .fetch_one(&mut *tx)
    .await?;

    let revision = sqlx::query_as::<_, CmsRevisionRow>(
        r#"
        insert into admin_cms_revisions (
          id, item_id, version, payload, summary, created_by_kind, created_by_user_id
        ) values ($1, $2, $3, $4, $5, $6, $7)
        returning id, item_id, version, payload, summary, created_by_kind, created_by_user_id, created_at
        "#,
    )
    .bind(revision_id)
    .bind(item_id)
    .bind(next_version)
    .bind(revision_payload)
    .bind(&summary)
    .bind(&actor.actor_kind)
    .bind(actor.actor_user_id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        update admin_cms_items
        set current_revision_id = $2,
            title = coalesce($3, title),
            status = coalesce($4, status),
            updated_by_kind = $5,
            updated_by_user_id = $6,
            updated_at = now()
        where id = $1
        "#,
    )
    .bind(item_id)
    .bind(revision_id)
    .bind(new_title)
    .bind(new_status)
    .bind(&actor.actor_kind)
    .bind(actor.actor_user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        insert into admin_cms_publish_events (
          id, item_id, revision_id, action, actor_kind, actor_user_id, reason, metadata
        ) values ($1, $2, $3, 'revise', $4, $5, $6, $7)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(item_id)
    .bind(revision_id)
    .bind(&actor.actor_kind)
    .bind(actor.actor_user_id)
    .bind(&summary)
    .bind(json!({"version": next_version}))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    admin_auth::audit(
        &state,
        &actor,
        "cms_item_revise",
        &item.kind,
        actor.actor_user_id,
        Some(item_id),
        &actor.capabilities,
        "CMS item revision was created.",
        json!({"revision_id": revision_id, "version": next_version}),
    )
    .await?;

    let item = fetch_item(&state, item_id).await?;
    Ok(success(ItemWriteData { item, revision }))
}

#[derive(Debug, Deserialize)]
struct StatusActionRequest {
    reason: Option<String>,
}

async fn publish_item(
    Path(item_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<StatusActionRequest>,
) -> Result<Json<AdminEnvelope<CmsItemRow>>, ApiError> {
    let actor = admin_auth::authorize_with_capability(&state, &headers, "content_publish").await?;
    let reason = clean_summary(payload.reason.as_deref())?;
    let item = set_item_status(&state, item_id, "published", &actor, &reason).await?;
    admin_auth::audit(
        &state,
        &actor,
        "cms_item_publish",
        &item.kind,
        actor.actor_user_id,
        Some(item_id),
        &actor.capabilities,
        "CMS item was published.",
        json!({"slug": item.slug, "reason": reason}),
    )
    .await?;
    Ok(success(item))
}

async fn archive_item(
    Path(item_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<StatusActionRequest>,
) -> Result<Json<AdminEnvelope<CmsItemRow>>, ApiError> {
    let actor = admin_auth::authorize_with_capability(&state, &headers, "content_archive").await?;
    let reason = clean_summary(payload.reason.as_deref())?;
    let item = set_item_status(&state, item_id, "archived", &actor, &reason).await?;
    admin_auth::audit(
        &state,
        &actor,
        "cms_item_archive",
        &item.kind,
        actor.actor_user_id,
        Some(item_id),
        &actor.capabilities,
        "CMS item was archived.",
        json!({"slug": item.slug, "reason": reason}),
    )
    .await?;
    Ok(success(item))
}

async fn set_item_status(
    state: &AppState,
    item_id: Uuid,
    status: &str,
    actor: &admin_auth::AdminContext,
    reason: &str,
) -> Result<CmsItemRow, ApiError> {
    let item = fetch_item(state, item_id).await?;
    if item.current_revision_id.is_none() {
        return Err(ApiError::BadRequest("CMS item has no revision to publish".to_string()));
    }
    let event_action = if status == "published" { "publish" } else { "archive" };
    sqlx::query(
        r#"
        update admin_cms_items
        set status = $2,
            updated_by_kind = $3,
            updated_by_user_id = $4,
            published_at = case when $2 = 'published' then now() else published_at end,
            archived_at = case when $2 = 'archived' then now() else null end,
            updated_at = now()
        where id = $1
        "#,
    )
    .bind(item_id)
    .bind(status)
    .bind(&actor.actor_kind)
    .bind(actor.actor_user_id)
    .execute(&state.db)
    .await?;

    sqlx::query(
        r#"
        insert into admin_cms_publish_events (
          id, item_id, revision_id, action, actor_kind, actor_user_id, reason, metadata
        ) values ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(item_id)
    .bind(item.current_revision_id)
    .bind(event_action)
    .bind(&actor.actor_kind)
    .bind(actor.actor_user_id)
    .bind(reason)
    .bind(json!({"status": status}))
    .execute(&state.db)
    .await?;

    fetch_item(state, item_id).await
}

async fn fetch_item(state: &AppState, item_id: Uuid) -> Result<CmsItemRow, ApiError> {
    sqlx::query_as::<_, CmsItemRow>(
        r#"
        select item.id,
               item.kind,
               item.slug,
               item.title,
               item.status,
               item.current_revision_id,
               rev.version as current_version,
               item.published_at,
               item.archived_at,
               item.created_at,
               item.updated_at
        from admin_cms_items item
        left join admin_cms_revisions rev on rev.id = item.current_revision_id
        where item.id = $1
        "#,
    )
    .bind(item_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("CMS item was not found".to_string()))
}

async fn fetch_revision(state: &AppState, revision_id: Uuid) -> Result<Option<CmsRevisionRow>, ApiError> {
    sqlx::query_as::<_, CmsRevisionRow>(
        r#"
        select id, item_id, version, payload, summary, created_by_kind, created_by_user_id, created_at
        from admin_cms_revisions
        where id = $1
        "#,
    )
    .bind(revision_id)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::from)
}

fn normalize_kind(input: &str) -> Result<String, ApiError> {
    match input.trim() {
        "core_lesson" | "lab" => Ok(input.trim().to_string()),
        _ => Err(ApiError::BadRequest("CMS kind must be core_lesson or lab".to_string())),
    }
}

fn normalize_status(input: &str) -> Result<String, ApiError> {
    match input.trim() {
        "draft" | "review" | "published" | "archived" => Ok(input.trim().to_string()),
        _ => Err(ApiError::BadRequest(
            "CMS status must be draft, review, published, or archived".to_string(),
        )),
    }
}

fn normalize_initial_status(input: &str) -> Result<String, ApiError> {
    match input.trim() {
        "draft" | "review" | "published" => Ok(input.trim().to_string()),
        _ => Err(ApiError::BadRequest(
            "initial CMS status must be draft, review, or published".to_string(),
        )),
    }
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

fn clean_title(input: &str) -> Result<String, ApiError> {
    let value = input.trim();
    if value.is_empty() || value.chars().count() > 180 {
        return Err(ApiError::BadRequest("title is required and must be at most 180 characters".to_string()));
    }
    if value.chars().any(char::is_control) {
        return Err(ApiError::BadRequest("title cannot contain control characters".to_string()));
    }
    Ok(value.to_string())
}

fn clean_summary(input: Option<&str>) -> Result<String, ApiError> {
    let value = input.unwrap_or("").trim();
    if value.chars().count() > 1000 {
        return Err(ApiError::BadRequest("summary/reason is too long".to_string()));
    }
    if value.chars().any(char::is_control) {
        return Err(ApiError::BadRequest("summary/reason cannot contain control characters".to_string()));
    }
    Ok(value.to_string())
}

fn clean_search(input: Option<&str>) -> Result<Option<String>, ApiError> {
    let Some(value) = input else { return Ok(None) };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > 120 || value.chars().any(char::is_control) {
        return Err(ApiError::BadRequest("search query is invalid".to_string()));
    }
    Ok(Some(value.to_string()))
}

fn clean_payload(value: Value) -> Result<Value, ApiError> {
    if !value.is_object() {
        return Err(ApiError::BadRequest("CMS payload must be a JSON object".to_string()));
    }
    Ok(value)
}

fn clean_metadata(value: Value) -> Result<Value, ApiError> {
    if !value.is_object() {
        return Err(ApiError::BadRequest("CMS metadata must be a JSON object".to_string()));
    }
    Ok(value)
}
