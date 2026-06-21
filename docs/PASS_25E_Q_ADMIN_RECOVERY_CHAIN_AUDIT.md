# PASS 25E-Q — Admin Recovery Chain Audit & Runbook

Status: audit/runbook checkpoint after password recovery execution.

## Purpose

This pass freezes and audits the current admin recovery chain after password recovery execution was introduced.

The goal is to make sure Karyra Spark recovery stays hierarchical, single-use, audited, and separated by recovery type before adding email or 2FA recovery execution.

## Current supported recovery execution

Implemented:

- Recovery request submission through `/admin/reset`.
- Hierarchical review through `/admin/reset/requests`.
- Recovery artifact issuance after approval.
- Recovery artifact inspection through `/admin/recovery`.
- Password recovery execution through `/api/admin/recovery/password`.

Not implemented yet:

- Email recovery execution.
- 2FA recovery execution.
- Superadmin account recovery automation.

## End-to-end password recovery chain

1. Admin/moderator submits reset request at `/admin/reset`.
2. Backend returns a neutral response and does not reveal whether the email exists.
3. Reviewer opens `/admin/reset/requests`.
4. Superadmin can review admin/moderator requests.
5. Admin can review moderator requests only.
6. Moderator cannot review recovery requests.
7. Approved request can issue a recovery artifact.
8. Recovery artifact token is stored as hash only.
9. Raw artifact token is returned only through guarded bootstrap mode.
10. Affected admin opens `/admin/recovery`.
11. Artifact inspection verifies token hash, email, status, expiry, used/revoked state.
12. Password recovery requires artifact type `password`, fresh password, and current TOTP code.
13. Password recovery hashes the new password with Argon2.
14. Password recovery marks the artifact `used` exactly once.
15. Password recovery marks the reset request `completed`.
16. Password recovery revokes existing delegated admin sessions.
17. Password recovery emits `admin_recovery_password_completed` audit event.

## Security invariants

- `/admin/reset` must remain unauthenticated but non-enumerating.
- `/admin/recovery` must remain unauthenticated but token-gated.
- `/admin/reset/requests` must remain authenticated and role-scoped.
- Review approval must not directly mutate credentials.
- Artifact issuance must not directly mutate credentials.
- Artifact inspection must not mutate credentials or artifact status.
- Password recovery must only work with `request_type = password`.
- Password recovery must require current TOTP.
- Password recovery must consume artifact exactly once.
- Password recovery must mark the reset request completed.
- Password recovery must revoke old delegated admin sessions.
- Email and 2FA recovery must not be exposed until separately implemented and audited.

## Manual test checklist

### Neutral request

- Submit `/admin/reset` with a non-admin email.
- Submit `/admin/reset` with a real admin email.
- Both should show the same neutral response.

### Review boundary

- Superadmin can see admin/moderator reset requests.
- Admin can see only moderator reset requests.
- Moderator cannot access `/admin/reset/requests`.

### Artifact issuance

- Pending request can be approved/rejected.
- Approved request can issue an artifact.
- A second pending artifact for the same request is rejected.
- Artifact token is only visible if bootstrap return mode is enabled.

### Artifact intake

- Valid artifact + matching email shows metadata.
- Invalid token fails with generic error.
- Mismatched email fails with generic error.
- Expired/used/revoked artifact fails with generic error.

### Password recovery

- Password artifact + wrong TOTP fails.
- Password artifact + valid TOTP + fresh password succeeds.
- Used artifact cannot be reused.
- Reset request becomes `completed`.
- Existing admin sessions are revoked.
- New password works only after normal admin login with TOTP.

## Next safe order

1. PASS 25E-R: 2FA recovery rotation design lock.
2. PASS 25E-S: 2FA recovery setup/confirm flow.
3. PASS 25E-T: email recovery design lock.
4. PASS 25E-U: email recovery flow, only after out-of-band verification model is finalized.
