create table if not exists admin_public_rate_limit_events (
  id uuid primary key,
  scope text not null,
  subject_hash text not null,
  allowed boolean not null,
  occurred_at timestamptz not null default now(),
  metadata jsonb not null default '{}'::jsonb
);

create index if not exists admin_public_rate_limit_events_scope_subject_time_idx
  on admin_public_rate_limit_events(scope, subject_hash, occurred_at desc);

create index if not exists admin_public_rate_limit_events_time_idx
  on admin_public_rate_limit_events(occurred_at desc);
