use serde_json::json;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::error::ApiError;

pub struct RecoveryNotification<'a> {
    pub user_id: Option<Uuid>,
    pub event_type: &'a str,
    pub recipient_email: &'a str,
    pub subject: &'a str,
    pub body: &'a str,
    pub related_artifact_id: Option<Uuid>,
    pub related_reset_request_id: Option<Uuid>,
    pub metadata: serde_json::Value,
}

pub async fn enqueue_recovery_notification_tx(
    tx: &mut Transaction<'_, Postgres>,
    notification: RecoveryNotification<'_>,
) -> Result<(), ApiError> {
    let recipient_email = notification.recipient_email.trim().to_ascii_lowercase();
    if recipient_email.is_empty() || !recipient_email.contains('@') {
        return Ok(());
    }

    sqlx::query(
        r#"
        insert into admin_recovery_notification_outbox (
          id, user_id, event_type, channel, recipient_email, subject, body, status,
          related_artifact_id, related_reset_request_id, metadata
        ) values ($1, $2, $3, 'email', $4, $5, $6, 'pending', $7, $8, $9)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(notification.user_id)
    .bind(notification.event_type)
    .bind(recipient_email)
    .bind(notification.subject)
    .bind(notification.body)
    .bind(notification.related_artifact_id)
    .bind(notification.related_reset_request_id)
    .bind(notification.metadata)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub fn recovery_notification_metadata(
    source: &str,
    mutation_type: &str,
    notification_delivery_pending: bool,
) -> serde_json::Value {
    json!({
        "source": source,
        "mutation_type": mutation_type,
        "notification_delivery_pending": notification_delivery_pending
    })
}
