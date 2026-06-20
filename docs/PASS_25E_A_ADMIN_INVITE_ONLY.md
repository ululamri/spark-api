# PASS 25E-A — Invite-only admin model

This pass introduces the backend invite boundary for delegated Spark admin users.

## Policy

- Superadmin can invite `admin` and `moderator`.
- Admin can invite `moderator` only.
- Moderator cannot invite anyone.
- Direct delegated role creation is disabled from `POST /api/admin/team/members`.
- Delegated role activation must happen through invite-token onboarding in the next pass.

## Backend routes

```txt
GET  /api/admin/team/invitations
POST /api/admin/team/invitations
POST /api/admin/team/invitations/:invitation_id/revoke
```

Existing member read/revoke routes remain:

```txt
GET  /api/admin/team/members
POST /api/admin/team/members/:user_id/revoke
```

## Schema

Migration:

```txt
migrations/202606200003_admin_invite_only_model.sql
```

Adds:

- `admin_invitations`
- `admin_invite_email_otps`
- `admin_reset_requests`

Invitation tokens are stored as hashes in `admin_invitations.token_hash`. A raw token is only returned when the temporary bootstrap flag is enabled:

```env
SPARK_ADMIN_INVITE_RETURN_BOOTSTRAP_TOKENS=true
```

This flag is for early manual delivery only and must be removed when production email delivery is active.

## Audit

```bash
node scripts/audit-pass25e-a-admin-invite-only.mjs
```

Expected:

```txt
PASS 25E-A admin invite-only model audit
OK: delegated admin/moderator role activation is invite-first; superadmin can invite admin/moderator, admin can invite moderator only, moderator cannot invite.
```

## Deployment note

Apply migration before restart:

```bash
cd /opt/karyra/spark-api
set -a
source .env.host
set +a
psql "$DATABASE_URL" -f migrations/202606200003_admin_invite_only_model.sql
cargo build --release
systemctl restart karyra-spark-api
systemctl status karyra-spark-api --no-pager
```
