# PASS 25E-L — Admin Recovery Execution Lock

Status: policy lock / no credential mutation implemented in this pass.

## Purpose

This document freezes the allowed execution model for admin recovery before any credential-changing endpoint is added.

Karyra Spark admin recovery must not become a direct takeover surface. Reset request review is only an approval record. It must not directly change password, email, or 2FA state from the review screen.

## Authority model

- Superadmin is the root authority.
- Admin is delegated operational authority.
- Moderator is moderation-only delegated authority.
- Superadmin can review admin and moderator recovery requests.
- Admin can review moderator recovery requests only.
- Admin cannot approve another admin's recovery request.
- Admin cannot approve their own admin recovery request.
- Moderator cannot review recovery requests.

## Approved recovery execution model

When implementation proceeds beyond approval/rejection, credential recovery must use a controlled recovery flow:

1. A reset request is submitted through `/admin/reset`.
2. The request receives a neutral response and does not confirm whether the account exists.
3. The request is reviewed through `/admin/reset/requests`.
4. Approval records audit evidence only.
5. A separate recovery artifact may be created only after approval.
6. The recovery artifact must be single-use, short-lived, and stored as a hash only.
7. The affected admin must complete a recovery flow themselves.
8. The recovery flow must require a fresh password and fresh 2FA setup when 2FA recovery is involved.
9. Credential changes must be audited with actor, target role, request ID, and recovery artifact ID.

## Explicitly forbidden

The following patterns are forbidden:

- Direct password change button on the review page.
- Direct email replacement button on the review page.
- Direct 2FA disable button on the review page.
- Returning raw recovery token after normal approval unless a guarded bootstrap mode is explicitly enabled.
- Storing raw recovery tokens.
- Allowing admin to approve admin recovery.
- Allowing moderator to review any recovery request.
- Revealing whether an email is an admin account on `/admin/reset`.

## Future multi-superadmin note

For public launch, Karyra Spark may have multiple superadmins from the dev team. Multi-superadmin support must preserve this hierarchy:

- Superadmin can review all admin/moderator recovery requests.
- Superadmin account recovery should require a separate dev-team process, not delegated admin approval.
- A superadmin recovery flow should require at least one independent superadmin or an offline root procedure.

## Current implementation state

Implemented now:

- Invite-only admin/moderator onboarding.
- Split superadmin login and delegated admin login.
- Public unauthenticated reset request surface with neutral response.
- Hierarchical reset review queue.
- Single-use, short-lived recovery artifact issuance after approval.
- Audit guards for invite, onboarding, reset, navigation, recovery artifacts, and security boundary.

Not implemented yet:

- Actual credential recovery execution.
- Recovery artifact consumption.
- Password/email/2FA mutation from recovery flow.
- Multi-superadmin database-backed root authority.
