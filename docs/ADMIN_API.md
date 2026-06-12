# Admin Backend API v1

The Admin API is a read-only dashboard interface mounted at /api/admin. Existing /v1 public and learner routes are unchanged.

## Authentication

Set a long, random KARYRA_ADMIN_TOKEN in the API runtime environment. Every request must include the x-karyra-admin-token header.

If the environment variable is missing or empty, admin routes return HTTP 503 with admin_not_configured. A missing or incorrect request token returns HTTP 401 with admin_unauthorized.

The token is never returned or logged. Browser requests remain restricted to the configured SPARK_WEB_ORIGIN; CORS permits the admin header for that origin.

This token is a bootstrap control, not production identity management.

- TODO: replace it with scoped admin RBAC tied to authenticated identities.
- TODO: add audit events for admin access and sensitive data reads.
- TODO: add rate limiting and token rotation procedures.

## Routes

All routes are GET and read-only.

| Route | Current source |
| --- | --- |
| /api/admin/overview | Users, progress, lab, Passport, proof, and workshop tables |
| /api/admin/learners?limit=50&offset=0 | Users, profiles, sessions, progress, and Passport tables |
| /api/admin/learners/:id | Learner profile and related progress/evidence records |
| /api/admin/lessons | Observed lesson_progress identifiers only |
| /api/admin/lab | Lab attempts |
| /api/admin/passports | Passport credentials |
| /api/admin/proof-ledger | System proof events |
| /api/admin/community-pilot | Workshop participant count plus typed placeholders |
| /api/admin/starknet | Typed not_configured response |
| /api/admin/system | Safe service/configuration and database health fields |

Example request:

    curl -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN" http://127.0.0.1:8787/api/admin/overview

## Response Shape

Success:

    {
      "ok": true,
      "data": {},
      "generated_at": "2026-06-12T00:00:00Z"
    }

Error:

    {
      "ok": false,
      "error": {
        "code": "admin_unauthorized",
        "message": "Admin access is not authorized."
      }
    }

## Data Limitations

- There is no backend lesson catalog. /lessons reports only lesson IDs observed in progress records; title and catalog status fields remain null or unavailable.
- There is no cohort or pilot-session model. /community-pilot reports the distinct active workshop-registration participant count and empty typed cohort/session arrays.
- No Starknet RPC reader or network configuration exists in the API. /starknet reports not_configured and never exposes RPC URLs.
- Proof-ledger records are backend evidence records. starknet_attestation_status reflects stored Passport anchor status or none; it does not claim an onchain attestation.
- Learner email is returned only through this protected admin surface. Downstream exports should minimize or omit it unless operationally necessary.
- List endpoints are capped at 100 records per request.

## Safety Boundary

Admin API v1 does not add wallet connection, wallet autoconnect, signature prompts, transaction prompts, private-key handling, seed-phrase handling, RPC writes, or any other onchain write path. It does not alter learner/public behavior.

