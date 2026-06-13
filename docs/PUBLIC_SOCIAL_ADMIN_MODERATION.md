# Public Social Admin Moderation

**Pass:** PUBLIC-SOCIAL-03  
**Repository:** `spark-api`  
**Mount:** `/api/admin/social`

This pass adds a small admin moderation surface for the API-backed public social layer.

It uses the same bootstrap admin token as the existing admin dashboard:

```text
x-karyra-admin-token: <KARYRA_ADMIN_TOKEN>
```

## Routes

```text
GET  /api/admin/social/scope
GET  /api/admin/social/reports
GET  /api/admin/social/posts
GET  /api/admin/social/comments
POST /api/admin/social/moderation-actions
```

## List reports

```bash
curl -s \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN" \
  "https://spark.user.cloudjkt01.com/api/admin/social/reports?status=pending&limit=20" | jq
```

Optional query params:

```text
status=pending|reviewed|actioned|dismissed
target_type=post|comment|profile|media
limit=1..100
offset=0..
```

## List posts

```bash
curl -s \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN" \
  "https://spark.user.cloudjkt01.com/api/admin/social/posts?status=published&limit=20" | jq
```

Optional status values:

```text
published
hidden
removed
deleted
```

## List comments

```bash
curl -s \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN" \
  "https://spark.user.cloudjkt01.com/api/admin/social/comments?status=published&limit=20" | jq
```

## Create moderation action

### Hide a post

```bash
curl -s -X POST \
  -H "content-type: application/json" \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN" \
  -d '{
    "target_type": "post",
    "target_id": "POST_UUID",
    "action": "hide",
    "reason": "Admin moderation review"
  }' \
  "https://spark.user.cloudjkt01.com/api/admin/social/moderation-actions" | jq
```

### Remove a comment and mark a report actioned

```bash
curl -s -X POST \
  -H "content-type: application/json" \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN" \
  -d '{
    "target_type": "comment",
    "target_id": "COMMENT_UUID",
    "action": "remove",
    "reason": "Confirmed report",
    "report_id": "REPORT_UUID"
  }' \
  "https://spark.user.cloudjkt01.com/api/admin/social/moderation-actions" | jq
```

### Restore a post

```bash
curl -s -X POST \
  -H "content-type: application/json" \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN" \
  -d '{
    "target_type": "post",
    "target_id": "POST_UUID",
    "action": "restore",
    "reason": "Restored after review"
  }' \
  "https://spark.user.cloudjkt01.com/api/admin/social/moderation-actions" | jq
```

### Dismiss a report

```bash
curl -s -X POST \
  -H "content-type: application/json" \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN" \
  -d '{
    "target_type": "report",
    "target_id": "REPORT_UUID",
    "action": "dismiss_report",
    "reason": "No violation found"
  }' \
  "https://spark.user.cloudjkt01.com/api/admin/social/moderation-actions" | jq
```

## Action rules

Valid content actions:

```text
post/comment + hide
post/comment + remove
post/comment + restore
```

Valid report actions:

```text
report + dismiss_report
report + mark_reviewed
```

## Notes

- This pass does not add a new migration. It uses tables from `0069_public_social_schema.sql`.
- `social_moderation_actions.moderator_user_id` is currently `null` because admin auth still uses the bootstrap token, not scoped admin user identities.
- A future RBAC pass should replace token-only moderation with authenticated admin accounts and audit identity.
