# Admin Audit Trail & Observability Runbook

Status: operational hardening runbook  
Scope: Karyra Spark admin audit API, admin auth audit writer, social moderation audit coverage  
Mode: no schema mutation, no fake data

## Purpose

This runbook keeps the admin surface debuggable and accountable while delegated admin testing is deferred.

The expected chain is:

```txt
superadmin / admin / moderator action
-> shared admin auth context
-> capability check
-> domain mutation when applicable
-> admin_audit_events row
-> /api/admin/audit/events readable by audit_read
-> /admin/audit UI/debug surface
```

## Current audit surfaces

Backend:

```txt
GET /api/admin/audit/scope
GET /api/admin/audit/events
GET /api/admin/audit/events/:event_id
```

Required capability:

```txt
audit_read
```

Core audit event fields:

```txt
id
actor_kind
actor_user_id
action
target_type
target_user_id
target_id
capabilities
summary
metadata
created_at
```

## Observability expectations

1. Audit API must be protected by `audit_read`.
2. Superadmin must have `audit_read`.
3. Admin role may have `audit_read`.
4. Moderator should not receive `audit_read` by default unless intentionally granted later.
5. Audit list must support paging.
6. Audit list must support safe filters:
   - actor_kind
   - action
   - target_type
7. Audit detail must return one event by ID.
8. Filters must reject control characters and excessive length.
9. Social moderation single actions must write audit events.
10. Social bulk moderation must write job-level and item-level audit events.
11. Audit metadata must include enough context to debug:
   - target_type
   - target_id
   - action
   - capability
   - report_id when relevant
   - dry_run/status/counts for bulk actions

## Manual smoke checklist

Use superadmin first, delegated admin later when team testing is available.

### 1. Scope check

```bash
curl -s "$SPARK_API_BASE/api/admin/audit/scope" \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN" | jq .
```

Expected:

```txt
ok = true
data.auth_model mentions superadmin/delegated audit_read
routes include scope/events/events/:event_id
```

### 2. Events list check

```bash
curl -s "$SPARK_API_BASE/api/admin/audit/events?limit=10" \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN" | jq .
```

Expected:

```txt
ok = true
data.items is array
data.limit <= 100
data.data_source = database
```

### 3. Filter check

```bash
curl -s "$SPARK_API_BASE/api/admin/audit/events?target_type=post&limit=5" \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN" | jq .
```

Expected:

```txt
ok = true
no server error
filter does not leak unrelated target_type
```

### 4. Detail check

Pick one event ID from the list:

```bash
curl -s "$SPARK_API_BASE/api/admin/audit/events/<event_id>" \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN" | jq .
```

Expected:

```txt
ok = true
data.id matches event_id
metadata is present
created_at is present
```

### 5. Negative auth check

Run without admin token:

```bash
curl -i -s "$SPARK_API_BASE/api/admin/audit/events?limit=1"
```

Expected:

```txt
401 Unauthorized
no event data returned
```

## Pass criteria

PASS only if:

```txt
audit routes exist
audit_read is enforced
audit writer inserts admin_audit_events
single social moderation action writes audit
bulk moderation job writes job audit
bulk moderation item writes item audit
filter/paging guardrails exist
cargo build succeeds
```

## Deferred check

Delegated admin/moderator manual verification is deferred until team/user accounts are available.
This is not a blocker while superadmin smoke is verified and audit coverage is structurally present.

## Failure handling

If audit smoke fails:

```txt
capture route
capture response status
capture response body
capture last backend logs
do not patch blindly
patch only the smallest broken boundary
```
