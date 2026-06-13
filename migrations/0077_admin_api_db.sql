create extension if not exists pgcrypto;

create table if not exists users (
  id uuid primary key default gen_random_uuid(),
  email text unique,
  password_hash text,
  status text not null default 'active',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists profiles (
  user_id uuid primary key references users(id) on delete cascade,
  display_name text,
  handle text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists sessions (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  token_hash text,
  created_at timestamptz not null default now(),
  expires_at timestamptz,
  revoked_at timestamptz,
  last_seen_at timestamptz
);

create table if not exists lesson_progress (
  user_id uuid not null references users(id) on delete cascade,
  lesson_id text not null,
  level text not null default 'foundation',
  status text not null default 'started',
  progress_percent integer not null default 0,
  completed_at timestamptz,
  updated_at timestamptz not null default now(),
  primary key (user_id, lesson_id)
);

create table if not exists lab_attempts (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  lab_id text not null,
  level text not null default 'foundation',
  status text not null default 'started',
  score integer,
  safety_score integer,
  started_at timestamptz not null default now(),
  completed_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists checkpoint_results (
  id uuid primary key default gen_random_uuid(),
  user_id uuid references users(id) on delete cascade,
  track text not null,
  checkpoint_id text not null,
  status text not null default 'recorded',
  created_at timestamptz not null default now()
);

create table if not exists exam_attempts (
  id uuid primary key default gen_random_uuid(),
  user_id uuid references users(id) on delete cascade,
  track text not null,
  exam_id text not null,
  status text not null default 'recorded',
  score integer,
  created_at timestamptz not null default now()
);

create table if not exists passport_credentials (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  readiness_level text not null default 'foundation',
  issue_status text not null default 'draft',
  evidence_event_count bigint not null default 0,
  starknet_anchor_status text not null default 'none',
  evidence_root text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists proof_events (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  event_type text not null,
  subject_id text not null,
  source_table text,
  issuer text not null default 'karyra-spark',
  event_hash text not null default encode(gen_random_bytes(32), 'hex'),
  created_at timestamptz not null default now()
);

create table if not exists community_workshop_registrations (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  workshop_id text not null,
  status text not null default 'registered',
  registered_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_sessions_user_id on sessions(user_id);
create index if not exists idx_lesson_progress_user_id on lesson_progress(user_id);
create index if not exists idx_lesson_progress_lesson_id on lesson_progress(lesson_id);
create index if not exists idx_lab_attempts_user_id on lab_attempts(user_id);
create index if not exists idx_lab_attempts_lab_id on lab_attempts(lab_id);
create index if not exists idx_checkpoint_results_track on checkpoint_results(track);
create index if not exists idx_exam_attempts_track on exam_attempts(track);
create index if not exists idx_passport_credentials_user_id on passport_credentials(user_id);
create index if not exists idx_proof_events_user_id_created_at on proof_events(user_id, created_at desc);
create index if not exists idx_community_workshop_user_id on community_workshop_registrations(user_id);
create index if not exists idx_community_workshop_status on community_workshop_registrations(status);