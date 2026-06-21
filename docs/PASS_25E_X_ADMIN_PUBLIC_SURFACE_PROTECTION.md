# PASS 25E-X — Admin Unauthenticated Surface Protection

Status: backend hardening batch.

## Purpose

This pass adds database-backed throttling for unauthenticated admin surfaces:

- public reset request,
- recovery artifact intake/execution endpoints,
- invite-only onboarding endpoints.

## Scope

Protected backend surfaces:

- `POST /api/admin/reset/request`
- `POST /api/admin/recovery/inspect`
- `POST /api/admin/recovery/password`
- `POST /api/admin/recovery/totp/setup`
- `POST /api/admin/recovery/totp/confirm`
- `POST /api/admin/recovery/email/request`
- `POST /api/admin/recovery/email/confirm`
- `POST /api/admin/recovery/email/complete`
- `POST /api/admin/onboarding/invite/inspect`
- `POST /api/admin/onboarding/invite/email/request`
- `POST /api/admin/onboarding/invite/email/confirm`
- `POST /api/admin/onboarding/invite/password`
- `POST /api/admin/onboarding/invite/totp/setup`
- `POST /api/admin/onboarding/invite/totp/confirm`
- `POST /api/admin/onboarding/invite/accept`

## Model

The guard stores event rows in `admin_public_rate_limit_events` using:

- scope,
- hashed subject,
- allowed/blocked state,
- timestamp,
- non-sensitive metadata.

The subject hash is derived from request network hints, user-agent, and the submitted public subject such as email/token. Raw email/token/IP/user-agent values are not stored in the rate-limit table.

## Notes

This is application-level throttling. It complements, but does not replace, reverse-proxy/WAF-level rate limiting.
