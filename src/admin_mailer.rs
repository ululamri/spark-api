use std::time::Duration;

use anyhow::{anyhow, Context};
use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{config::AppConfig, state::AppState};

#[derive(Clone)]
struct SmtpDeliveryConfig {
    host: String,
    port: u16,
    security: String,
    username: String,
    password: String,
    from: Mailbox,
    interval_seconds: u64,
    batch_limit: i64,
}

#[derive(Debug, FromRow)]
struct PendingNotification {
    id: Uuid,
    recipient_email: String,
    subject: String,
    body: String,
}

pub fn spawn_smtp_delivery_worker(state: AppState) -> anyhow::Result<()> {
    let Some(config) = SmtpDeliveryConfig::from_app_config(&state.config)? else {
        return Ok(());
    };

    let transport = build_transport(&config)?;
    let interval_seconds = config.interval_seconds.max(10);
    let batch_limit = config.batch_limit.clamp(1, 50);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_seconds));
        loop {
            interval.tick().await;
            if let Err(error) = deliver_pending_batch(&state, &transport, &config.from, batch_limit).await {
                tracing::error!(?error, "admin recovery SMTP delivery worker failed");
            }
        }
    });

    tracing::info!(
        smtp_host = %config.host,
        smtp_port = config.port,
        smtp_security = %config.security,
        "admin recovery SMTP delivery worker started"
    );

    Ok(())
}

impl SmtpDeliveryConfig {
    fn from_app_config(config: &AppConfig) -> anyhow::Result<Option<Self>> {
        let driver = config.mail_driver.trim().to_ascii_lowercase();
        if driver.is_empty() {
            return Ok(None);
        }

        if driver != "smtp" {
            return Err(anyhow!("unsupported SPARK_MAIL_DRIVER: {driver}; only smtp is implemented"));
        }

        let host = required(config.smtp_host.clone(), "SPARK_SMTP_HOST")?;
        let username = required(config.smtp_username.clone(), "SPARK_SMTP_USERNAME")?;
        let password = required(config.smtp_password.clone(), "SPARK_SMTP_PASSWORD")?;
        let mail_from = required(config.mail_from.clone(), "SPARK_MAIL_FROM")?;
        let from = parse_from_mailbox(&mail_from, &config.mail_from_name)?;

        Ok(Some(Self {
            host,
            port: config.smtp_port,
            security: config.smtp_security.clone(),
            username,
            password,
            from,
            interval_seconds: config.mail_worker_interval_seconds,
            batch_limit: config.mail_worker_batch_limit,
        }))
    }
}

fn required(value: Option<String>, key: &str) -> anyhow::Result<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("{key} is required when SPARK_MAIL_DRIVER=smtp"))
}

fn parse_from_mailbox(email: &str, name: &str) -> anyhow::Result<Mailbox> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        email.parse().with_context(|| format!("invalid SPARK_MAIL_FROM: {email}"))
    } else {
        format!("{trimmed_name} <{email}>")
            .parse()
            .with_context(|| format!("invalid SPARK_MAIL_FROM/SPARK_MAIL_FROM_NAME: {trimmed_name} <{email}>"))
    }
}

fn build_transport(config: &SmtpDeliveryConfig) -> anyhow::Result<AsyncSmtpTransport<Tokio1Executor>> {
    let credentials = Credentials::new(config.username.clone(), config.password.clone());
    let builder = match config.security.as_str() {
        "starttls" => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
            .with_context(|| format!("failed to build STARTTLS SMTP transport for {}", config.host))?,
        "tls" | "smtps" => AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
            .with_context(|| format!("failed to build TLS SMTP transport for {}", config.host))?,
        other => {
            return Err(anyhow!(
                "unsupported SPARK_SMTP_SECURITY: {other}; use starttls for Gmail port 587"
            ))
        }
    };

    Ok(builder.port(config.port).credentials(credentials).build())
}

async fn deliver_pending_batch(
    state: &AppState,
    transport: &AsyncSmtpTransport<Tokio1Executor>,
    from: &Mailbox,
    batch_limit: i64,
) -> anyhow::Result<()> {
    let items = sqlx::query_as::<_, PendingNotification>(
        r#"
        select id, recipient_email, subject, body
        from admin_recovery_notification_outbox
        where status = 'pending'
        order by created_at asc
        limit $1
        "#,
    )
    .bind(batch_limit)
    .fetch_all(&state.db)
    .await?;

    for item in items {
        match send_notification(transport, from, &item).await {
            Ok(()) => {
                sqlx::query(
                    r#"
                    update admin_recovery_notification_outbox
                    set status = 'sent',
                        sent_at = now(),
                        failure_reason = null
                    where id = $1 and status = 'pending'
                    "#,
                )
                .bind(item.id)
                .execute(&state.db)
                .await?;
            }
            Err(error) => {
                let reason = error.to_string().chars().take(1000).collect::<String>();
                sqlx::query(
                    r#"
                    update admin_recovery_notification_outbox
                    set status = 'failed',
                        failed_at = now(),
                        failure_reason = $2
                    where id = $1 and status = 'pending'
                    "#,
                )
                .bind(item.id)
                .bind(reason)
                .execute(&state.db)
                .await?;
            }
        }
    }

    Ok(())
}

async fn send_notification(
    transport: &AsyncSmtpTransport<Tokio1Executor>,
    from: &Mailbox,
    item: &PendingNotification,
) -> anyhow::Result<()> {
    let to: Mailbox = item
        .recipient_email
        .parse()
        .with_context(|| format!("invalid outbox recipient email for {}", item.id))?;

    let email = Message::builder()
        .from(from.clone())
        .to(to)
        .subject(&item.subject)
        .header(ContentType::TEXT_PLAIN)
        .body(item.body.clone())?;

    transport
        .send(email)
        .await
        .with_context(|| format!("SMTP send failed for outbox notification {}", item.id))?;

    Ok(())
}
