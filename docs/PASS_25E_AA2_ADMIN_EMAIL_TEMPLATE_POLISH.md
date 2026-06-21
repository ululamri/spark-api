# PASS 25E-AA2 — Admin Email Template Polish

This pass improves admin auth email copy for invitation, onboarding OTP, and recovery notifications.

## Template direction

All admin auth emails now use a consistent plain-text branded structure:

- `Karyra Spark Admin Panel` header
- short title
- warm greeting: `Halo Sahabat Karyra,`
- concise purpose-specific message
- clear link or OTP
- security note
- professional footer

## Scope

- Admin invitation email
- Admin onboarding OTP email
- Password recovery completion notice
- 2FA recovery completion notice
- Email recovery old-address notice
- Email recovery new-address notice

## Boundary

This pass does not introduce UI settings yet. Future superadmin settings can make these templates configurable.

There is no dry-run, mock delivery, simulated delivery, or fake sent state. Emails continue to use the real Gmail SMTP worker through the durable outbox.
