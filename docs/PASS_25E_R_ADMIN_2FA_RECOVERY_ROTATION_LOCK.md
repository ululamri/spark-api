# PASS 25E-R — Admin 2FA Recovery Rotation Lock

Status: design lock and audit checkpoint. No 2FA credential mutation is implemented in this pass.

## Purpose

This pass freezes the allowed model for admin 2FA recovery before any TOTP recovery execution endpoint is added.

Karyra Spark must not implement 2FA recovery as a simple “disable 2FA” action. That pattern would create an account takeover path. 2FA recovery must be a rotation flow: verify approved artifact, require fresh password, create a new pending TOTP factor, confirm the new code, then revoke the old factor only after the new factor is active.

## Current state

Implemented before this pass:

- `/admin/reset` submits non-enumerating recovery requests.
- `/admin/reset/requests` provides hierarchical review.
- Approved requests may issue single-use recovery artifacts.
- `/admin/recovery` can inspect recovery artifacts.
- Password recovery execution exists for `request_type = password`.
- Password recovery requires current TOTP, consumes the artifact, marks the reset request completed, and revokes existing delegated admin sessions.

Not implemented in this pass:

- 2FA recovery execution.
- 2FA factor revocation from recovery artifact.
- New 2FA setup from recovery artifact.
- Email recovery execution.

## Required 2FA recovery model

A future 2FA recovery flow must follow this order:

1. The reset request must be `request_type = totp`.
2. The reset request must be approved through the existing hierarchy.
3. A recovery artifact must be issued for the approved request.
4. The affected admin must enter artifact token + matching email at `/admin/recovery`.
5. The affected admin must verify their current password.
6. Backend creates a new pending TOTP factor.
7. Backend returns an otpauth URI and manual secret for the new factor.
8. The affected admin must confirm the new TOTP code.
9. Only after new factor confirmation, backend revokes old active TOTP factors.
10. Backend marks the artifact `used` exactly once.
11. Backend marks the reset request `completed`.
12. Backend revokes existing delegated admin sessions.
13. Backend emits audit events for setup, confirmation, old-factor revocation, and completion.

## Explicitly forbidden

The following are forbidden:

- Direct `disable 2FA` button on review page.
- Direct `revoke TOTP` from `/admin/reset/requests`.
- Any recovery endpoint that sets `enabled_at = null` on the only active factor without confirming a replacement.
- Any recovery endpoint that sets `revoked_at = now()` before a new factor is confirmed.
- Any route named `/api/admin/recovery/totp/disable`.
- Any route named `/api/admin/recovery/totp/revoke`.
- Any unauthenticated route that mutates 2FA without artifact token + email + password + new TOTP confirmation.

## Future endpoint shape

Allowed future endpoints should be split into setup and confirm:

- `POST /api/admin/recovery/totp/setup`
- `POST /api/admin/recovery/totp/confirm`

The setup endpoint may create only a pending factor. The confirm endpoint may enable the pending factor, revoke old active factors, consume the artifact, complete the reset request, revoke sessions, and audit the final mutation.

## Security invariants

- No old factor is revoked until replacement is confirmed.
- Artifact must remain hash-only and single-use.
- Generic invalid/expired error behavior must be preserved.
- Admin hierarchy remains in review/issuance layer.
- A moderator cannot review recovery requests.
- Admin cannot approve admin recovery.
- Superadmin recovery remains outside delegated admin recovery automation.
