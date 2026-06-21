create table if not exists admin_recovery_artifacts (
  id uuid primary key,
  reset_request_id uuid not null references admin_reset_requests(id) on delete cascade,
  email text not null,
  request_type text not null check (request_type in ('password', 'email', 'totp')),
  target_role text check (target_role in ('admin', 'moderator')),
  token_hash text not null unique,
  status text not null default 'pending' check (status in ('pending', 'used', 'revoked', 'expired')),
  created_by_actor_kind text not null,
  created_by_user_id uuid,
  issued_at timestamptz not null default now(),
  expires_at timestamptz not null,
  used_at timestamptz,
  revoked_at timestamptz,
  metadata jsonb not null default '{}'::jsonb
);

create index if not exists admin_recovery_artifacts_request_idx on admin_recovery_artifacts(reset_request_id);
create index if not exists admin_recovery_artifacts_email_idx on admin_recovery_artifacts(lower(email));
create index if not exists admin_recovery_artifacts_status_idx on admin_recovery_artifacts(status, expires_at);
create index if not exists admin_recovery_artifacts_created_by_idx on admin_recovery_artifacts(created_by_actor_kind, created_by_user_id);
