-- Karyra Spark Directus experiment schema
-- Scope: Learn + Lab content studio only.
-- Do not use this schema for users, sessions, social, proof ledger, passport, or moderation runtime tables.

create extension if not exists pgcrypto;

create table if not exists karyra_courses (
  id uuid primary key default gen_random_uuid(),
  status text not null default 'draft' check (status in ('draft', 'review', 'needs_changes', 'approved', 'published', 'archived')),
  slug text not null unique,
  title text not null,
  summary text not null default '',
  audience text not null default 'pemula',
  level text not null default 'beginner' check (level in ('beginner', 'intermediate', 'advanced')),
  sort_order integer not null default 100,
  user_created uuid,
  date_created timestamptz not null default now(),
  user_updated uuid,
  date_updated timestamptz not null default now()
);

create table if not exists karyra_lessons (
  id uuid primary key default gen_random_uuid(),
  course_id uuid references karyra_courses(id) on delete set null,
  status text not null default 'draft' check (status in ('draft', 'review', 'needs_changes', 'approved', 'published', 'archived')),
  slug text not null unique,
  title text not null,
  subtitle text not null default '',
  summary text not null default '',
  learning_goal text not null default '',
  estimated_minutes integer not null default 10 check (estimated_minutes > 0 and estimated_minutes <= 240),
  difficulty text not null default 'beginner' check (difficulty in ('beginner', 'intermediate', 'advanced')),
  sort_order integer not null default 100,
  published_version integer,
  published_at timestamptz,
  user_created uuid,
  date_created timestamptz not null default now(),
  user_updated uuid,
  date_updated timestamptz not null default now()
);

create table if not exists karyra_lesson_blocks (
  id uuid primary key default gen_random_uuid(),
  lesson_id uuid not null references karyra_lessons(id) on delete cascade,
  status text not null default 'draft' check (status in ('draft', 'review', 'needs_changes', 'approved', 'published', 'archived')),
  block_type text not null check (block_type in (
    'story', 'concept', 'analogy', 'media', 'code', 'checkpoint', 'quiz',
    'glossary', 'reflection', 'callout', 'ai_helper_prompt'
  )),
  title text not null default '',
  body text not null default '',
  payload jsonb not null default '{}'::jsonb,
  sort_order integer not null default 100,
  renderer_contract_version integer not null default 1,
  user_created uuid,
  date_created timestamptz not null default now(),
  user_updated uuid,
  date_updated timestamptz not null default now()
);

create table if not exists karyra_lab_runtime_profiles (
  id uuid primary key default gen_random_uuid(),
  status text not null default 'draft' check (status in ('draft', 'review', 'needs_changes', 'approved', 'published', 'archived')),
  slug text not null unique,
  title text not null,
  runtime_type text not null check (runtime_type in (
    'browser_only', 'shell', 'cairo', 'scarb', 'starknet_foundry', 'dojo', 'node', 'rust', 'plugin'
  )),
  sdk_profile text not null default '',
  tool_requirements jsonb not null default '[]'::jsonb,
  allowed_commands jsonb not null default '[]'::jsonb,
  network_policy text not null default 'disabled' check (network_policy in ('disabled', 'restricted', 'enabled')),
  filesystem_policy text not null default 'ephemeral' check (filesystem_policy in ('none', 'ephemeral', 'persistent')),
  command_timeout_seconds integer not null default 30 check (command_timeout_seconds > 0 and command_timeout_seconds <= 600),
  user_created uuid,
  date_created timestamptz not null default now(),
  user_updated uuid,
  date_updated timestamptz not null default now()
);

create table if not exists karyra_lab_modules (
  id uuid primary key default gen_random_uuid(),
  status text not null default 'draft' check (status in ('draft', 'review', 'needs_changes', 'approved', 'published', 'archived')),
  slug text not null unique,
  title text not null,
  summary text not null default '',
  learning_goal text not null default '',
  runtime_profile_id uuid references karyra_lab_runtime_profiles(id) on delete set null,
  estimated_minutes integer not null default 20 check (estimated_minutes > 0 and estimated_minutes <= 360),
  difficulty text not null default 'beginner' check (difficulty in ('beginner', 'intermediate', 'advanced')),
  prerequisite_notes text not null default '',
  published_version integer,
  published_at timestamptz,
  user_created uuid,
  date_created timestamptz not null default now(),
  user_updated uuid,
  date_updated timestamptz not null default now()
);

create table if not exists karyra_lab_steps (
  id uuid primary key default gen_random_uuid(),
  lab_module_id uuid not null references karyra_lab_modules(id) on delete cascade,
  status text not null default 'draft' check (status in ('draft', 'review', 'needs_changes', 'approved', 'published', 'archived')),
  step_type text not null check (step_type in (
    'instruction', 'task', 'shell', 'code', 'quiz', 'checkpoint', 'hint',
    'expected_output', 'safety_note', 'ai_helper_prompt'
  )),
  title text not null default '',
  instruction text not null default '',
  starter_files jsonb not null default '[]'::jsonb,
  validation_mode text not null default 'manual' check (validation_mode in ('manual', 'text_match', 'command_exit_code', 'script', 'api_check')),
  validation_payload jsonb not null default '{}'::jsonb,
  expected_output text not null default '',
  hints jsonb not null default '[]'::jsonb,
  safety_notes text not null default '',
  sort_order integer not null default 100,
  renderer_contract_version integer not null default 1,
  user_created uuid,
  date_created timestamptz not null default now(),
  user_updated uuid,
  date_updated timestamptz not null default now()
);

create table if not exists karyra_content_ai_reviews (
  id uuid primary key default gen_random_uuid(),
  status text not null default 'open' check (status in ('open', 'accepted', 'rejected', 'superseded')),
  target_collection text not null check (target_collection in ('karyra_lessons', 'karyra_lesson_blocks', 'karyra_lab_modules', 'karyra_lab_steps')),
  target_id uuid not null,
  review_type text not null check (review_type in ('clarity', 'beginner_friendliness', 'safety', 'technical_accuracy', 'quiz_quality', 'lab_runtime', 'publish_readiness')),
  score numeric(5,2),
  summary text not null default '',
  recommendations jsonb not null default '[]'::jsonb,
  provider text not null default 'spark_api_admin_ai',
  model_name text not null default '',
  created_by_ai boolean not null default true,
  user_created uuid,
  date_created timestamptz not null default now(),
  user_updated uuid,
  date_updated timestamptz not null default now()
);

create table if not exists karyra_publish_events (
  id uuid primary key default gen_random_uuid(),
  target_collection text not null check (target_collection in ('karyra_courses', 'karyra_lessons', 'karyra_lab_modules')),
  target_id uuid not null,
  action text not null check (action in ('submit_review', 'request_changes', 'approve', 'publish', 'archive', 'restore')),
  reason text not null default '',
  metadata jsonb not null default '{}'::jsonb,
  user_created uuid,
  date_created timestamptz not null default now()
);

create index if not exists karyra_lessons_status_idx on karyra_lessons(status);
create index if not exists karyra_lesson_blocks_lesson_sort_idx on karyra_lesson_blocks(lesson_id, sort_order);
create index if not exists karyra_lab_modules_status_idx on karyra_lab_modules(status);
create index if not exists karyra_lab_steps_module_sort_idx on karyra_lab_steps(lab_module_id, sort_order);
create index if not exists karyra_content_ai_reviews_target_idx on karyra_content_ai_reviews(target_collection, target_id);
create index if not exists karyra_publish_events_target_idx on karyra_publish_events(target_collection, target_id);
