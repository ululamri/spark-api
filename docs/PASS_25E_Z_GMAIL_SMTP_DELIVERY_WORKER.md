# PASS 25E-Z — Gmail SMTP Delivery Worker

This pass adds real SMTP delivery for `admin_recovery_notification_outbox`.

## Gmail configuration

Use a dedicated Gmail account and a Google App Password, not the normal account password.

Required env:

```env
SPARK_MAIL_DRIVER=smtp
SPARK_SMTP_HOST=smtp.gmail.com
SPARK_SMTP_PORT=587
SPARK_SMTP_SECURITY=starttls
SPARK_SMTP_USERNAME=your-karyra-gmail@gmail.com
SPARK_SMTP_PASSWORD=your-16-digit-app-password
SPARK_MAIL_FROM=your-karyra-gmail@gmail.com
SPARK_MAIL_FROM_NAME=Karyra Spark
SPARK_MAIL_WORKER_INTERVAL_SECONDS=30
SPARK_MAIL_WORKER_BATCH_LIMIT=10
```

## Boundary

There is no dry-run, mock delivery, simulated delivery, or UI delivery state.

When `SPARK_MAIL_DRIVER=smtp`, missing SMTP env fails startup. When SMTP is configured, pending outbox rows are sent through the real SMTP server. Successful sends become `sent`; real send failures become `failed` with `failure_reason`.
