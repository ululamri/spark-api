# Admin Moderation Report Lifecycle Smoke Test

Status: operational smoke plan  
Scope: social reports, admin moderation queue, moderation action, audit trail  
Mode: no demo seed, no fake data, no schema mutation

## Purpose

Verify this full moderation lifecycle:

```txt
user report post/comment
-> social_reports
-> admin moderation queue
-> moderation action / bulk action
-> social_moderation_actions
-> admin audit trail
-> frontend admin moderation UI refresh
```

## Required backend surfaces

```txt
POST /v1/social/posts/:post_id/report
POST /v1/social/comments/:comment_id/report
GET  /api/admin/social/reports
GET  /api/admin/social/posts
GET  /api/admin/social/comments
POST /api/admin/social/moderation-actions
POST /api/admin/social/bulk/moderation-actions
GET  /api/admin/social/ops/bulk-jobs
GET  /api/admin/audit/events
```

## Required frontend surfaces

```txt
/community
/admin/moderation
/admin/audit
```

## Preconditions

1. Spark API is running.
2. Spark frontend is running.
3. At least one normal user account exists.
4. A normal user can create a social post/comment from UI.
5. Superadmin or delegated admin/moderator access is available.
6. `/admin/moderation` opens.
7. `/admin/audit` opens.

## Test account rule

Use real test accounts only through normal auth/UI flow.

Do not create users or posts directly through SQL.  
Do not insert social reports directly through SQL.  
Do not seed demo data through migrations.

## Lifecycle A — post report to reviewed

### 1. Create or choose a post

From `/community`, create a normal low-risk test post from a normal user account.

Record:

```txt
post_id:
author_user:
created_at:
```

### 2. Report the post

From another normal user session, report the post through UI.

Expected backend state:

```txt
social_reports.target_type = post
social_reports.target_id = post_id
social_reports.status = pending
```

Read-only DB check:

```sql
select id, target_type, target_id, reason, status, created_at
from social_reports
where target_type = 'post'
order by created_at desc
limit 10;
```

### 3. Verify admin queue

Open:

```txt
/admin/moderation?report_status=pending&report_target_type=post
```

Expected:

```txt
report appears in pending queue
target type is post
target id matches reported post
available actions are visible according to capability
```

### 4. Dry-run bulk mark reviewed

In admin moderation UI, select the report and run bulk report action with dry-run enabled.

Expected:

```txt
bulk job status is dry_run or completed dry-run equivalent
would_apply_count > 0
applied_count = 0
social_reports.status remains pending
```

### 5. Apply mark reviewed

Run the same report action without dry-run:

```txt
target_type = report
action = mark_reviewed
```

Expected backend state:

```txt
social_reports.status = reviewed
social_reports.reviewed_at is not null
social_reports.action_id is not null when action persistence is linked
social_moderation_actions has a report action row
admin audit event exists
```

Read-only DB check:

```sql
select id, target_type, target_id, status, reviewed_at, action_id, updated_at
from social_reports
where target_type = 'post'
order by updated_at desc
limit 10;
```

## Lifecycle B — post report to content action

### 1. Report a post

Use a real report as in Lifecycle A.

### 2. Apply content moderation

From `/admin/moderation`, select the post target and apply:

```txt
target_type = post
action = hide
```

Expected backend state:

```txt
social_posts.status = hidden
social_moderation_actions.target_type = post
social_moderation_actions.action = hide
admin audit event exists
```

Read-only DB check:

```sql
select id, status, updated_at
from social_posts
where id = '<post_id>';
```

### 3. Restore post

Apply:

```txt
target_type = post
action = restore
```

Expected backend state:

```txt
social_posts.status = published
social_moderation_actions.action = restore
admin audit event exists
```

## Lifecycle C — comment report

Repeat the same path using:

```txt
target_type = comment
```

Expected:

```txt
comment report appears in admin queue
comment status can move hidden/removed/published
report can be reviewed/dismissed
audit event exists
```

## UI refresh expectations

After each action:

```txt
/admin/moderation should refresh or show new state after submit
pending reviewed/dismissed filters must not show stale queue state
post/comment status filter must reflect backend state
bulk job history must show latest job
```

## Safety expectations

```txt
normal user cannot access /admin/moderation
moderator can access moderation queue only if capability allows
admin token is never visible in browser page source
superadmin token remains server-side only
```

## Pass criteria

PASS only if:

```txt
report creation uses normal user flow
report appears in admin queue
dry-run does not mutate report/content state
real action mutates only expected target
report status and content status stay consistent
moderation action is persisted
audit event is visible
admin UI reflects updated backend state
no SQL seed/fake data is used
```

## Failure handling

If smoke fails:

```txt
do not patch blindly
capture route, action, target id, response status, and latest backend logs
check social_reports, social_moderation_actions, social_posts/social_comments, and admin_audit_events
patch the smallest broken boundary only
```
