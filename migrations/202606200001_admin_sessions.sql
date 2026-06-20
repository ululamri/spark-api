create table if not exists admin_sessions (
  id uuid primary key,
  user_id uuid not null references users(id) on delete cascade,
  token_hash text not null unique,
  role text not null check (role in ('admin', 'moderator')),
  capabilities text[] not null default '{}',
  expires_at timestamptz not null,
  last_seen_at timestamptz not null default now(),
  revoked_at timestamptz,
  created_at timestamptz not null default now(),
  metadata jsonb not null default '{}'::jsonb
);

create index if not exists idx_admin_sessions_user_active
  on admin_sessions (user_id, expires_at desc)
  where revoked_at is null;

create index if not exists idx_admin_sessions_last_seen
  on admin_sessions (last_seen_at desc);
