# PASS 25E-U — Admin Email Recovery Proof Shell

Status: implemented as proof shell only. No account email mutation is implemented in this pass.

## Purpose

This pass prepares email recovery safely without changing `users.email`.

It creates a new-email OTP proof layer that a future final email recovery pass can consume.

## Backend surface

- `POST /api/admin/recovery/email/request`
- `POST /api/admin/recovery/email/confirm`

## Flow

1. The admin has an approved `request_type = email` recovery artifact.
2. The admin submits artifact token, current account email, account password, current TOTP code, and new email.
3. Backend verifies artifact hash, email match, password, and TOTP.
4. Backend validates that the new email is different and not already used by an active user.
5. Backend creates an OTP proof record in `admin_email_recovery_otps`.
6. Backend returns a manual OTP only when `SPARK_ADMIN_EMAIL_RECOVERY_RETURN_BOOTSTRAP_TOKENS=true`.
7. OTP confirmation consumes the OTP record and returns an email proof token.
8. No account email mutation happens in this pass.
9. Recovery artifact remains pending.
10. Reset request remains approved.

## Explicitly not implemented

- `users.email` update.
- `admin_recovery_email_completed`.
- artifact consumption for email recovery.
- reset request completion for email recovery.
- notification delivery.

## Next pass

PASS 25E-V should implement final email mutation using the proof token, then consume artifact, complete reset request, revoke sessions, and emit final audit.
