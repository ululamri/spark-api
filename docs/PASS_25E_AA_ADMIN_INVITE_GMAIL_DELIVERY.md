# PASS 25E-AA — Admin Invite + Onboarding Gmail Delivery

This pass completes the missing email delivery path for invite-only delegated admin onboarding.

## What is delivered

The existing Gmail SMTP worker reads `admin_recovery_notification_outbox`. This pass reuses that durable outbox for admin auth notifications:

- `admin_invitation_created_email`
- `admin_invite_email_otp_email`

## Invite delivery

When a superadmin/admin creates an invitation, the backend now queues a real email containing the onboarding link:

`{SPARK_WEB_ORIGIN}/admin/onboarding?token=...`

The invitation token is still stored as a hash only. The raw token exists only long enough to place it into the queued email body.

## OTP delivery

When an invitee requests the onboarding email OTP, the backend now queues a real email containing the OTP.

The OTP hash is stored in `admin_invite_email_otps`. The raw OTP exists only long enough to place it into the queued email body.

## Boundary

There is no dry-run, mock delivery, simulated delivery, or UI-only delivery.

The SMTP worker sends queued rows through the configured Gmail SMTP account and marks them `sent` or `failed`.
