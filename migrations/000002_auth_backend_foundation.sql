-- Karyra Spark API Pass 46: Auth backend foundation.
-- Adds the stable auth/session details needed by register/login/me/logout.

create unique index if not exists idx_users_email_lower on users (lower(email));

alter table sessions add column if not exists revoked_at timestamptz;
alter table sessions add column if not exists last_seen_at timestamptz;

create index if not exists idx_sessions_user_expires_at on sessions(user_id, expires_at desc);
create index if not exists idx_sessions_token_hash_active on sessions(token_hash) where revoked_at is null;

create table if not exists auth_audit_events (
  id uuid primary key default gen_random_uuid(),
  user_id uuid references users(id) on delete set null,
  event_type text not null,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now()
);

create index if not exists idx_auth_audit_events_user_created on auth_audit_events(user_id, created_at desc);
