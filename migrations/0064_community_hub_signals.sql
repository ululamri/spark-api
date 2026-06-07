create table if not exists community_workshop_registrations (
  id uuid primary key,
  user_id uuid not null references users(id) on delete cascade,
  workshop_id text not null,
  status text not null default 'registered',
  registered_at timestamptz not null default now(),
  canceled_at timestamptz,
  payload jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint community_workshop_status_check check (status in ('registered', 'canceled'))
);

create unique index if not exists community_workshop_registrations_user_workshop_idx
  on community_workshop_registrations(user_id, workshop_id);

create index if not exists community_workshop_registrations_user_status_idx
  on community_workshop_registrations(user_id, status, updated_at desc);

create table if not exists hub_resource_saves (
  id uuid primary key,
  user_id uuid not null references users(id) on delete cascade,
  resource_id text not null,
  status text not null default 'saved',
  saved_at timestamptz not null default now(),
  unsaved_at timestamptz,
  payload jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint hub_resource_save_status_check check (status in ('saved', 'unsaved'))
);

create unique index if not exists hub_resource_saves_user_resource_idx
  on hub_resource_saves(user_id, resource_id);

create index if not exists hub_resource_saves_user_status_idx
  on hub_resource_saves(user_id, status, updated_at desc);
