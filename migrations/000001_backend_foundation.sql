-- Karyra Spark API backend foundation.
-- This migration is intentionally broad but shallow: it defines stable boundaries
-- for auth, learning, lab, media, proof, and Passport records without enabling
-- production flows yet.

create extension if not exists pgcrypto;

create table if not exists users (
  id uuid primary key default gen_random_uuid(),
  email text unique not null,
  password_hash text,
  status text not null default 'active',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists profiles (
  user_id uuid primary key references users(id) on delete cascade,
  display_name text not null default '',
  handle text unique,
  bio text not null default '',
  location text not null default '',
  visibility text not null default 'community',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists sessions (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  token_hash text unique not null,
  expires_at timestamptz not null,
  created_at timestamptz not null default now()
);

create table if not exists exam_attempts (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  track text not null check (track in ('core', 'lab')),
  level text not null check (level in ('beginner', 'intermediate', 'advanced')),
  exam_id text not null,
  score integer not null check (score >= 0 and score <= 100),
  passed boolean not null,
  attempt_number integer not null default 1,
  exam_version text not null default 'v1',
  created_at timestamptz not null default now()
);

create table if not exists media_assets (
  id uuid primary key default gen_random_uuid(),
  owner_user_id uuid references users(id) on delete set null,
  bucket text not null,
  object_key text not null,
  mime_type text not null,
  size_bytes bigint not null check (size_bytes > 0),
  checksum text,
  visibility text not null default 'private',
  status text not null default 'pending',
  created_at timestamptz not null default now(),
  unique (bucket, object_key)
);

create table if not exists media_links (
  id uuid primary key default gen_random_uuid(),
  media_asset_id uuid not null references media_assets(id) on delete cascade,
  entity_type text not null,
  entity_id text not null,
  purpose text not null,
  created_at timestamptz not null default now()
);

create table if not exists proof_events (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  event_type text not null,
  subject_type text not null,
  subject_id text not null,
  level text,
  event_hash text,
  previous_event_hash text,
  issuer_signature text,
  payload jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now()
);

create table if not exists passport_credentials (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  readiness_level text not null check (readiness_level in ('beginner', 'intermediate', 'advanced')),
  verification_tier text not null default 'self_attested',
  issue_status text not null default 'draft',
  evidence_root text,
  starknet_anchor_status text not null default 'not_ready',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_exam_attempts_user_track_level on exam_attempts(user_id, track, level);
create index if not exists idx_media_assets_owner on media_assets(owner_user_id);
create index if not exists idx_proof_events_user on proof_events(user_id, created_at desc);
create index if not exists idx_passport_credentials_user on passport_credentials(user_id);
