# PASS 25E-Y — Admin Recovery Notification Outbox

Status: backend notification outbox integration.

## Purpose

This pass adds a durable notification outbox for admin recovery events. It does not send SMTP yet. It records pending email notifications so delivery can be connected later without losing security-relevant recovery events.

## Outbox table

`admin_recovery_notification_outbox`

Fields include user id, event type, channel, recipient email, subject/body, status, related artifact id, related reset request id, metadata, and sent/failed timestamps.

## Recovery events queued

- `admin_password_recovery_completed_notice`
- `admin_totp_recovery_completed_notice`
- `admin_email_recovery_old_email_notice`
- `admin_email_recovery_new_email_notice`

## Boundary

This pass does not claim notification delivery has happened. Outbox rows are created with status `pending`.

A future SMTP worker pass should pick pending rows, send email, then mark `sent` or `failed`.
