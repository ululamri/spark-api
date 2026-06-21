create table if not exists admin_recovery_notification_outbox (
  id uuid primary key,
  user_id uuid,
  event_type text not null,
  channel text not null default 'email',
  recipient_email text not null,
  subject text not null,
  body text not null,
  status text not null default 'pending'
    check (status in ('pending', 'sent', 'failed', 'skipped')),
  related_artifact_id uuid references admin_recovery_artifacts(id) on delete set null,
  related_reset_request_id uuid references admin_reset_requests(id) on delete set null,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  sent_at timestamptz,
  failed_at timestamptz,
  failure_reason text
);

create index if not exists admin_recovery_notification_outbox_status_idx
  on admin_recovery_notification_outbox(status, created_at);

create index if not exists admin_recovery_notification_outbox_related_idx
  on admin_recovery_notification_outbox(related_artifact_id, related_reset_request_id);

create index if not exists admin_recovery_notification_outbox_recipient_idx
  on admin_recovery_notification_outbox(lower(recipient_email), created_at desc);
