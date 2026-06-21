# PASS 25E-O — Admin Password Recovery Execution

Status: implemented for password recovery only.

## Purpose

This pass enables password recovery after the approved reset request and recovery artifact flow.

It does not implement email recovery or 2FA recovery execution.

## Backend endpoint

- `POST /api/admin/recovery/password`

Required input:

- recovery artifact token,
- admin email,
- fresh password,
- current TOTP code.

## Safety boundary

Password recovery only proceeds when:

- artifact exists by token hash,
- artifact email matches the supplied email,
- artifact status is `pending`,
- artifact has not expired,
- artifact is not used/revoked,
- artifact `request_type` is `password`,
- target admin/moderator account is still active,
- target role matches artifact target role,
- target has an enabled TOTP factor,
- submitted TOTP code is valid and not replayed.

## Effects

When successful:

- user password hash is replaced,
- artifact is marked `used`,
- active delegated admin sessions for the user are revoked,
- audit action `admin_recovery_password_completed` is recorded.

## Non-goals

This pass does not:

- change admin email,
- disable or reset 2FA,
- create a login session,
- allow password recovery without current TOTP,
- recover superadmin root credentials.
