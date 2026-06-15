# Admin RBAC Foundation

**Pass:** ADMIN-RBAC-01

Karyra Spark now has the backend foundation for three admin categories:

```text
super_admin
sub_admin
moderator
```

## Role model

`super_admin` is the existing bootstrap administrator with developer-level capability.

`sub_admin` is a delegated administrator with configurable capability below super-admin level.

`moderator` is focused on content review, reports, media review, and moderation actions.

## Capability catalog

Super-admin capability set:

```text
developer_access
admin_manage
policy_manage
ai_manage
moderation_read
moderation_action
moderation_restore
user_safety_manage
reports_manage
content_read
media_review
audit_read
```

Sub-admin allowed capabilities:

```text
policy_manage
ai_manage
moderation_read
moderation_action
moderation_restore
user_safety_manage
reports_manage
content_read
media_review
audit_read
```

Moderator allowed capabilities:

```text
moderation_read
moderation_action
reports_manage
content_read
media_review
audit_read
```

## Database tables

```text
admin_role_assignments
admin_audit_events
```

`admin_role_assignments` stores user-based admin role assignments, capability lists, status, optional expiry, and grant metadata.

`admin_audit_events` stores sensitive admin activity logs.

## Implemented files

```text
migrations/0083_admin_rbac_foundation.sql
src/admin_auth.rs
src/admin_team.rs
```

## Intended API

```text
GET  /api/admin/team/scope
GET  /api/admin/team/capabilities
GET  /api/admin/team/members
POST /api/admin/team/members
POST /api/admin/team/members/:user_id/revoke
```

## Next step

The next Admin UI pass should expose role-aware admin shell, member management, and capability check wiring.
