-- PASS 17E: Advanced social moderation bulk action engine.
-- Stores synchronous bulk moderation jobs, per-item results, dry-run output,
-- idempotency keys, and durable links to moderation actions/audit events.

create table if not exists social_moderation_bulk_jobs (
  id uuid primary key,
  actor_kind text not null,
  actor_user_id uuid references users(id) on delete set null,
  target_type text not null check (target_type in ('post', 'comment', 'report')),
  action text not null check (action in ('hide', 'remove', 'restore', 'dismiss_report', 'mark_reviewed')),
  reason text not null default '',
  status text not null default 'running' check (status in ('running', 'dry_run', 'completed', 'partial_failed', 'failed')),
  dry_run boolean not null default false,
  idempotency_key text,
  total_count integer not null default 0 check (total_count >= 0),
  would_apply_count integer not null default 0 check (would_apply_count >= 0),
  applied_count integer not null default 0 check (applied_count >= 0),
  skipped_count integer not null default 0 check (skipped_count >= 0),
  failed_count integer not null default 0 check (failed_count >= 0),
  payload jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  completed_at timestamptz
);

create unique index if not exists social_moderation_bulk_jobs_idempotency_key_idx
  on social_moderation_bulk_jobs(idempotency_key)
  where idempotency_key is not null;

create index if not exists social_moderation_bulk_jobs_created_at_idx
  on social_moderation_bulk_jobs(created_at desc);

create index if not exists social_moderation_bulk_jobs_actor_idx
  on social_moderation_bulk_jobs(actor_kind, actor_user_id, created_at desc);

create index if not exists social_moderation_bulk_jobs_status_idx
  on social_moderation_bulk_jobs(status, created_at desc);

create table if not exists social_moderation_bulk_job_items (
  id uuid primary key,
  bulk_job_id uuid not null references social_moderation_bulk_jobs(id) on delete cascade,
  target_type text not null check (target_type in ('post', 'comment', 'report')),
  target_id uuid not null,
  action text not null check (action in ('hide', 'remove', 'restore', 'dismiss_report', 'mark_reviewed')),
  status text not null check (status in ('would_apply', 'applied', 'skipped', 'failed')),
  action_id uuid references social_moderation_actions(id) on delete set null,
  report_id uuid references social_reports(id) on delete set null,
  message text not null default '',
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now()
);

create index if not exists social_moderation_bulk_job_items_job_idx
  on social_moderation_bulk_job_items(bulk_job_id, created_at asc);

create index if not exists social_moderation_bulk_job_items_target_idx
  on social_moderation_bulk_job_items(target_type, target_id, created_at desc);

create index if not exists social_moderation_bulk_job_items_action_idx
  on social_moderation_bulk_job_items(action_id);
