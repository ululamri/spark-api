# PASS 25E-T — Admin Email Recovery Lock

Status: design lock / no email recovery execution implemented in this pass.

## Purpose

This pass locks the email recovery model before any endpoint is allowed to mutate an admin account email.

Email recovery is higher risk than password recovery because the email is both an identifier and a recovery/contact channel. A weak email recovery flow can become full account takeover.

## Current state

Implemented:

- reset request submission,
- hierarchical review,
- recovery artifact issuance,
- artifact intake,
- password recovery execution,
- 2FA recovery rotation execution.

Not implemented:

- email recovery execution,
- recovery email OTP delivery,
- old-email/new-email notification delivery,
- delayed/cooldown email switch.

## Locked future email recovery model

Email recovery must not be implemented as a direct review action or as a raw `set email = ...` mutation from the review queue.

A future email recovery flow must require all of the following:

1. Approved `request_type = email` recovery artifact.
2. Artifact token + current account email match.
3. Account password verification.
4. Current TOTP verification if an enabled factor still exists.
5. New email syntax validation and normalization.
6. OTP/code proof sent to the new email address.
7. New email is updated only after the new-email OTP is confirmed.
8. Old email and new email receive notifications when the recovery is completed.
9. Recovery artifact is marked `used` exactly once.
10. Reset request is marked `completed`.
11. Existing delegated admin sessions are revoked.
12. Final mutation is audited as `admin_recovery_email_completed`.

## Explicitly forbidden

The following are forbidden until the future email recovery pass implements all required gates:

- `POST /api/admin/recovery/email`
- `set email =` inside `src/admin_recovery.rs`
- `change_email`
- `new_email` frontend field in `/admin/recovery`
- email mutation from `/admin/reset/requests`
- direct email change after approval without new-email OTP proof
- email recovery combined with password or 2FA recovery in one artifact

## External security basis

This design follows these principles:

- reset/recovery requests must avoid account enumeration,
- recovery tokens must be random, long, stored securely, single-use, and expiring,
- account changes should not happen until valid proof is presented,
- recovery addresses must be verified before being established,
- recovery events must notify the subscriber.

## Next safe implementation order

1. Add email recovery schema for OTP/proof records.
2. Add backend `/api/admin/recovery/email/request` for new-email OTP issuance.
3. Add backend `/api/admin/recovery/email/confirm` for final email mutation.
4. Add frontend two-step email recovery form.
5. Add audit scripts proving no direct email mutation exists outside the final confirm step.
6. Add manual test checklist for old-email/new-email notifications when delivery is available.
