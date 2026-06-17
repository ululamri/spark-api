-- PASS 17C: Admin CMS backend foundation for Learn/Core and Lab content.
-- Public website copy remains owned by ksbuilder. These tables are for
-- structured learning content with revision and publish workflow.

create table if not exists admin_cms_items (
  id uuid primary key,
  kind text not null check (kind in ('core_lesson', 'lab')),
  slug text not null,
  title text not null,
  status text not null default 'draft' check (status in ('draft', 'review', 'published', 'archived')),
  current_revision_id uuid,
  created_by_kind text not null,
  created_by_user_id uuid references users(id) on delete set null,
  updated_by_kind text not null,
  updated_by_user_id uuid references users(id) on delete set null,
  published_at timestamptz,
  archived_at timestamptz,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique(kind, slug)
);

create table if not exists admin_cms_revisions (
  id uuid primary key,
  item_id uuid not null references admin_cms_items(id) on delete cascade,
  version integer not null,
  payload jsonb not null default '{}'::jsonb,
  summary text not null default '',
  created_by_kind text not null,
  created_by_user_id uuid references users(id) on delete set null,
  created_at timestamptz not null default now(),
  unique(item_id, version)
);

create index if not exists admin_cms_items_kind_status_idx
  on admin_cms_items(kind, status, updated_at desc);

create index if not exists admin_cms_items_slug_idx
  on admin_cms_items(slug);

create index if not exists admin_cms_revisions_item_version_idx
  on admin_cms_revisions(item_id, version desc);

create table if not exists admin_cms_publish_events (
  id uuid primary key,
  item_id uuid not null references admin_cms_items(id) on delete cascade,
  revision_id uuid references admin_cms_revisions(id) on delete set null,
  action text not null check (action in ('create', 'revise', 'publish', 'unpublish', 'archive', 'restore')),
  actor_kind text not null,
  actor_user_id uuid references users(id) on delete set null,
  reason text not null default '',
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now()
);

create index if not exists admin_cms_publish_events_item_created_idx
  on admin_cms_publish_events(item_id, created_at desc);
