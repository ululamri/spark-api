# PASS 25E-V — Admin Email Recovery Finalization

Status: implemented for final email recovery mutation.

## Purpose

This pass completes admin email recovery after the new-email proof shell from PASS 25E-U.

## Backend surface

- `POST /api/admin/recovery/email/complete`

## Flow

1. Artifact must be pending and `request_type = email`.
2. Token hash and current email must match the recovery artifact.
3. New email must be different and still available.
4. New-email proof token must match a consumed OTP proof.
5. Proof token must not be expired.
6. Backend updates `users.email`.
7. Backend sets `email_verified_at` to the completion time.
8. Backend marks the artifact `used`.
9. Backend marks the reset request `completed`.
10. Backend revokes delegated admin sessions.
11. Backend emits `admin_recovery_email_completed`.

## Notes

Notification delivery is recorded as pending because outbound email delivery is not yet integrated in this recovery chain.

## Manual test checklist

- Complete with invalid proof token fails.
- Complete with expired proof token fails.
- Complete with already used artifact fails.
- Complete with unavailable new email fails.
- Complete with valid proof succeeds.
- Old email no longer logs in as delegated admin.
- New email logs in with existing password and active TOTP.
- Existing delegated admin sessions are revoked.
