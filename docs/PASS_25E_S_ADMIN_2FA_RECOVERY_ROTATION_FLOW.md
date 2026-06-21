# PASS 25E-S — Admin 2FA Recovery Rotation Flow

Status: implemented for 2FA recovery rotation only.

## Purpose

This pass implements 2FA recovery without introducing a direct "disable 2FA" button.

The recovery flow rotates the TOTP factor safely:

1. Verify approved recovery artifact with `request_type = totp`.
2. Require the affected admin's account password.
3. Create a fresh pending TOTP factor.
4. Ask the admin to confirm the new 6-digit TOTP code.
5. Revoke old enabled TOTP factors only after the new factor is confirmed.
6. Mark the recovery artifact `used`.
7. Mark the reset request `completed`.
8. Revoke existing delegated admin sessions.
9. Emit `admin_recovery_totp_completed`.

## Backend endpoints

- `POST /api/admin/recovery/totp/setup`
- `POST /api/admin/recovery/totp/confirm`

## Security properties

- No direct 2FA disable endpoint.
- Existing TOTP is not revoked during setup.
- Existing TOTP is revoked only after the replacement factor is verified.
- Artifact token is still hash-checked and single-use.
- Email recovery remains unimplemented.
- Password recovery remains unchanged.

## Manual test checklist

- TOTP artifact + wrong password fails.
- TOTP artifact + correct password creates a pending new factor.
- Old enabled factor still works until confirmation.
- Confirm with wrong new code fails.
- Confirm with correct new code enables the new factor.
- Old factors are revoked after successful confirmation.
- Artifact cannot be reused.
- Reset request becomes `completed`.
- Existing admin sessions are revoked.
