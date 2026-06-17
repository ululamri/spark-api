-- PASS 18A: Auth registration/login rate-limit guard.
-- Bucketed by privacy-preserving SHA-256 key hashes from email/client identity.

create table if not exists auth_rate_limits (
  bucket text not null,
  key_hash text not null,
  attempt_count integer not null default 0,
  window_expires_at timestamptz not null,
  first_seen_at timestamptz not null default now(),
  last_seen_at timestamptz not null default now(),
  metadata jsonb not null default '{}'::jsonb,
  primary key (bucket, key_hash)
);

create index if not exists auth_rate_limits_window_idx
  on auth_rate_limits(window_expires_at);
