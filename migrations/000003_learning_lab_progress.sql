-- Karyra Spark API Pass 47: Learning/Lab progress API.
-- These records are system-generated proof sources. They are not manual user claims.

alter table exam_attempts add column if not exists payload jsonb not null default '{}'::jsonb;

create table if not exists lesson_progress (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  lesson_id text not null,
  level text not null check (level in ('beginner', 'intermediate', 'advanced')),
  status text not null default 'in_progress' check (status in ('not_started', 'in_progress', 'completed')),
  progress_percent integer not null default 0 check (progress_percent >= 0 and progress_percent <= 100),
  completed_at timestamptz,
  payload jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (user_id, lesson_id)
);

create table if not exists checkpoint_results (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  track text not null check (track in ('core', 'lab')),
  subject_id text not null,
  checkpoint_id text not null,
  level text not null check (level in ('beginner', 'intermediate', 'advanced')),
  score integer not null check (score >= 0 and score <= 100),
  passed boolean not null,
  payload jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now()
);

create table if not exists lab_attempts (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  lab_id text not null,
  level text not null check (level in ('beginner', 'intermediate', 'advanced')),
  status text not null default 'submitted' check (status in ('started', 'submitted', 'passed', 'failed')),
  score integer check (score is null or (score >= 0 and score <= 100)),
  safety_score integer check (safety_score is null or (safety_score >= 0 and safety_score <= 100)),
  started_at timestamptz not null default now(),
  completed_at timestamptz,
  payload jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_lesson_progress_user_updated on lesson_progress(user_id, updated_at desc);
create index if not exists idx_lesson_progress_user_level on lesson_progress(user_id, level);
create index if not exists idx_checkpoint_results_user_track_level on checkpoint_results(user_id, track, level, created_at desc);
create index if not exists idx_lab_attempts_user_level_updated on lab_attempts(user_id, level, updated_at desc);
create index if not exists idx_exam_attempts_user_track_created on exam_attempts(user_id, track, created_at desc);
