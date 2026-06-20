create table if not exists admin_invitations (
  id uuid primary key,
  email text not null,
  role text not null check (role in ('admin', 'moderator')),
  capabilities text[] not null default '{}',
  token_hash text not null unique,
  invited_by_actor_kind text not null,
  invited_by_user_id uuid references users(id) on delete set null,
  expires_at timestamptz not null,
  accepted_at timestamptz,
  accepted_by_user_id uuid references users(id) on delete set null,
  revoked_at timestamptz,
  created_at timestamptz not null default now(),
  metadata jsonb not null default '{}'::jsonb,
  check (expires_at > created_at)
);

create index if not exists idx_admin_invitations_email_created
  on admin_invitations (lower(email), created_at desc);

create index if not exists idx_admin_invitations_pending
  on admin_invitations (role, expires_at desc)
  where accepted_at is null and revoked_at is null;

create index if not exists idx_admin_invitations_inviter
  on admin_invitations (invited_by_user_id, created_at desc);

create table if not exists admin_invite_email_otps (
  id uuid primary key,
  invitation_id uuid not null references admin_invitations(id) on delete cascade,
  email text not null,
  otp_hash text not null,
  expires_at timestamptz not null,
  attempt_count integer not null default 0,
  consumed_at timestamptz,
  created_at timestamptz not null default now(),
  metadata jsonb not null default '{}'::jsonb
);

create index if not exists idx_admin_invite_email_otps_active
  on admin_invite_email_otps (invitation_id, expires_at desc)
  where consumed_at is null;

create table if not exists admin_reset_requests (
  id uuid primary key,
  email text not null,
  request_type text not null check (request_type in ('password', 'email', 'totp')),
  requested_by_ip_hash text,
  user_agent_hash text,
  status text not null default 'pending' check (status in ('pending', 'approved', 'rejected', 'completed', 'expired')),
  requested_at timestamptz not null default now(),
  reviewed_by_actor_kind text,
  reviewed_by_user_id uuid references users(id) on delete set null,
  reviewed_at timestamptz,
  expires_at timestamptz not null,
  metadata jsonb not null default '{}'::jsonb
);

create index if not exists idx_admin_reset_requests_queue
  on admin_reset_requests (status, requested_at desc);

create index if not exists idx_admin_reset_requests_email
  on admin_reset_requests (lower(email), requested_at desc);
