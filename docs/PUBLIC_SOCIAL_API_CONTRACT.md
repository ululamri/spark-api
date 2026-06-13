# Public Social API Contract

**Pass:** PUBLIC-SOCIAL-01  
**Repository:** `spark-api`  
**Scope:** API-backed social feed, public profile cards, media attachments, follow state, viewer hide state, and moderation reporting.

This document is the implementation contract for replacing the current local-only social feed with a backend-backed public community feed.

## Goals

- Make posts, comments, and reactions visible across users.
- Keep author identity safe by exposing profile fields only, never email.
- Connect social content to uploaded media through the existing `media_assets` and `media_links` tables.
- Provide minimal reporting and moderation primitives before public release.
- Keep the first public feed simple: latest posts, pagination, comments, reactions, media attachments, profile cards, and report/hide actions.

## Database source of truth

The first schema pass is stored in:

```text
migrations/0069_public_social_schema.sql
```

Core tables:

```text
social_posts
social_comments
social_reactions
social_follows
social_post_hides
social_reports
social_moderation_actions
```

Existing tables used by the social layer:

```text
users
profiles
media_assets
media_links
```

## Privacy rules

Public social responses must not expose `users.email`.

Author cards should be hydrated from `profiles` only:

```json
{
  "user_id": "uuid",
  "display_name": "Budi",
  "handle": "@budi",
  "bio": "Learning Starknet safely",
  "location": "Indonesia",
  "visibility": "community",
  "avatar_preset": "spark",
  "avatar_url": null
}
```

Profiles with `visibility = private` should not be returned as public author cards. The feed can still show a safe fallback display label when needed.

## Media attachment strategy

Uploads remain owned by `media_assets`.

Social posts and comments attach media through `media_links`:

```text
entity_type = social_post
entity_id   = <social_posts.id as text>
purpose     = community
```

For comments:

```text
entity_type = social_comment
entity_id   = <social_comments.id as text>
purpose     = community
```

Only uploaded assets with allowed visibility should be included in public feed responses.

## Public social routes

### Scope

```text
GET /v1/social/scope
```

Returns the current backend phase and route contract.

### Feed

```text
GET /v1/social/feed?limit=20&cursor=<opaque>&kind=<optional>
```

Auth: optional session.

Returns latest visible posts.

Response shape:

```json
{
  "items": [
    {
      "post": {},
      "author": {},
      "media": [],
      "stats": {
        "comments": 0,
        "reactions": {}
      },
      "viewer": {
        "has_reacted": false,
        "reaction_kinds": [],
        "is_following_author": false,
        "is_hidden": false
      }
    }
  ],
  "next_cursor": null
}
```

### Posts

```text
POST /v1/social/posts
GET  /v1/social/posts/:post_id
POST /v1/social/posts/:post_id/hide
```

Create request:

```json
{
  "kind": "post",
  "body": "First public Spark note",
  "visibility": "community",
  "media_asset_ids": ["uuid"]
}
```

Rules:

- Auth is required to create.
- Body max length follows the database constraint.
- `media_asset_ids` must belong to the current user.
- Creating a post should create `media_links` rows for accepted attachments.
- A viewer hide action must only affect the current viewer.

### Comments

```text
POST /v1/social/posts/:post_id/comments
```

Request:

```json
{
  "body": "Helpful note",
  "parent_comment_id": null,
  "media_asset_ids": []
}
```

Rules:

- Auth is required.
- Comments can only be created for visible published posts.
- Comment media uses `entity_type = social_comment`.

### Reactions

```text
POST   /v1/social/posts/:post_id/reactions
DELETE /v1/social/posts/:post_id/reactions/:kind
POST   /v1/social/comments/:comment_id/reactions
DELETE /v1/social/comments/:comment_id/reactions/:kind
```

Request:

```json
{
  "kind": "like"
}
```

Allowed kinds:

```text
like
support
insightful
celebrate
```

### Public profiles and follows

```text
GET    /v1/social/profiles/:user_id
POST   /v1/social/profiles/:user_id/follow
DELETE /v1/social/profiles/:user_id/follow
```

Rules:

- `GET` can be optional-session.
- Follow/unfollow requires auth.
- A user cannot follow themselves.
- Email must never be exposed.

### Reports

```text
POST /v1/social/posts/:post_id/report
POST /v1/social/comments/:comment_id/report
```

Request:

```json
{
  "reason": "spam",
  "details": "Optional short explanation"
}
```

Allowed reasons:

```text
spam
abuse
harassment
unsafe
privacy
misleading
other
```

Reports enter `social_reports` with `status = pending`.

## Minimal admin moderation contract

The next admin pass should expose read-only queues first, then actions.

Suggested routes:

```text
GET  /api/admin/social/reports
GET  /api/admin/social/posts
GET  /api/admin/social/comments
POST /api/admin/social/moderation-actions
```

Suggested actions:

```text
hide
remove
restore
dismiss_report
mark_reviewed
```

A moderation action should write `social_moderation_actions`, then update the target row and related report status.

## Out of scope for this pass

- Direct messages.
- Real-time websocket updates.
- Algorithmic ranking.
- Public MinIO bucket exposure.
- Demo seed data.
