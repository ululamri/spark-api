alter table users
  add column if not exists email_verified_at timestamptz;

create table if not exists admin_email_verification_tokens (
  id uuid primary key,
  user_id uuid not null references users(id) on delete cascade,
  token_hash text not null unique,
  expires_at timestamptz not null,
  used_at timestamptz,
  attempt_count integer not null default 0,
  created_at timestamptz not null default now(),
  metadata jsonb not null default '{}'::jsonb
);

create index if not exists idx_admin_email_verification_tokens_user_active
  on admin_email_verification_tokens (user_id, expires_at desc)
  where used_at is null;

create table if not exists admin_totp_factors (
  id uuid primary key,
  user_id uuid not null references users(id) on delete cascade,
  label text not null default 'Authenticator app',
  secret_ciphertext bytea,
  secret_nonce bytea,
  secret_kdf text not null default 'spark-admin-v1',
  enabled_at timestamptz,
  verified_at timestamptz,
  revoked_at timestamptz,
  last_used_step bigint,
  recovery_codes_hashes text[] not null default '{}',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  metadata jsonb not null default '{}'::jsonb
);

create unique index if not exists idx_admin_totp_factors_one_active_per_user
  on admin_totp_factors (user_id)
  where enabled_at is not null and revoked_at is null;

create table if not exists admin_auth_challenges (
  id uuid primary key,
  user_id uuid not null references users(id) on delete cascade,
  challenge_type text not null check (challenge_type in ('email_verification', 'totp_setup', 'totp_login')),
  token_hash text not null unique,
  expires_at timestamptz not null,
  attempt_count integer not null default 0,
  consumed_at timestamptz,
  created_at timestamptz not null default now(),
  metadata jsonb not null default '{}'::jsonb
);

create index if not exists idx_admin_auth_challenges_user_active
  on admin_auth_challenges (user_id, challenge_type, expires_at desc)
  where consumed_at is null;
