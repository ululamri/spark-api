# PASS 25E-N — Admin Recovery Artifact Intake Shell

Status: implemented as inspect/intake only. No credential mutation is implemented in this pass.

## Purpose

This pass adds the first unauthenticated recovery artifact intake endpoint. The endpoint verifies that a recovery artifact exists, is pending, has not expired, and matches the supplied email.

It does not consume the artifact and does not change password, email, or 2FA.

## Added backend surface

- `POST /api/admin/recovery/inspect`

The request requires:

- raw recovery artifact token,
- admin email.

The backend checks:

- token prefix/shape,
- token hash,
- email match,
- artifact status is `pending`,
- artifact has not expired,
- `used_at` is null,
- `revoked_at` is null.

## Safety properties

- Token is verified by hash only.
- Invalid, expired, used, revoked, or mismatched artifacts return the same generic error.
- Inspection returns `credential_mutation: false`.
- Inspection does not update artifact state.
- Inspection does not change password, email, or 2FA.
- Actual recovery execution remains a separate future pass.

## Frontend surface

- `/admin/recovery`

The page accepts email + recovery artifact token and shows only artifact metadata needed to continue later.

## Next required pass

A future recovery execution flow may consume a valid artifact, but it must still:

1. mark artifact used exactly once,
2. require fresh password when password recovery is requested,
3. require fresh 2FA setup when TOTP recovery is requested,
4. audit the final credential mutation,
5. preserve hierarchy and neutral-error behavior.
