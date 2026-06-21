# PASS 25E-P — Admin Recovery Completion Finalization

Status: implemented for password recovery finalization only.

## Purpose

PASS 25E-O allowed approved password recovery artifacts to change a delegated admin/moderator password after TOTP verification. PASS 25E-P closes the lifecycle gap: once password recovery succeeds, the original reset request is marked `completed`.

Without this finalization, an approved reset request could remain reusable for issuing new recovery artifacts after one artifact had already been consumed.

## Backend behavior

When `POST /api/admin/recovery/password` succeeds:

1. recovery artifact is validated by token hash and email,
2. request type must be `password`,
3. current TOTP code is verified,
4. new password is hashed,
5. artifact is marked `used`,
6. original `admin_reset_requests` row is marked `completed`,
7. existing delegated admin sessions for the user are revoked,
8. audit `admin_recovery_password_completed` records `reset_request_completed: true`.

## Still not implemented

- Email recovery execution.
- 2FA recovery execution.
- Superadmin recovery execution.
