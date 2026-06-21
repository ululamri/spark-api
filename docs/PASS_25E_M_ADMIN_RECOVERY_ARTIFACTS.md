# PASS 25E-M — Admin Recovery Artifact Issuance

Status: implemented for artifact issuance only. No credential mutation is implemented in this pass.

## Purpose

This pass adds a controlled recovery artifact layer after reset review approval.

A recovery artifact is not a password reset. It is a single-use, short-lived, hashed token record that a future recovery flow can consume. This keeps approval and credential mutation separated.

## Added backend surface

- `POST /api/admin/reset/requests/:request_id/recovery-artifacts`

The route may issue a recovery artifact only when:

- the reset request exists,
- the reset request status is `approved`,
- the reset request has not expired,
- the reviewer is allowed by hierarchy,
- no active pending artifact already exists for the request.

## Hierarchy

- Superadmin may issue artifacts for approved admin/moderator requests.
- Admin may issue artifacts only for approved moderator requests.
- Admin may not issue artifacts for admin requests.
- Moderator may not issue artifacts.

## Artifact properties

- Stored in `admin_recovery_artifacts`.
- Token is stored as `token_hash` only.
- Raw token is returned only when `SPARK_ADMIN_RECOVERY_RETURN_BOOTSTRAP_TOKENS=true`.
- Default delivery mode is `out_of_band_delivery_pending`.
- Artifact expires after 45 minutes.
- A duplicate active artifact for the same reset request is rejected.
- Artifact issuance emits `admin_recovery_artifact_issue` audit event.

## Explicit non-goals of this pass

This pass does not implement:

- password replacement,
- email replacement,
- 2FA disable/reset,
- recovery token consumption,
- automatic credential mutation from the review page.

## Next required pass before credential mutation

Before any credential mutation exists, the project needs a separate recovery consumption flow that:

1. accepts the raw recovery artifact token,
2. verifies token hash and expiry,
3. validates target email/request type,
4. requires the affected admin to set a fresh password,
5. requires fresh 2FA setup for TOTP recovery,
6. marks the artifact used exactly once,
7. audits the final credential change.
