-- PASS 17B: Admin identity, roles, and audit foundation.
-- Idempotent by design so it can be applied safely on environments that already
-- received the early admin RBAC tables during development.

create table if not exists admin_role_assignments (
  id uuid primary key,
  user_id uuid not null references users(id) on delete cascade,
  role text not null check (role in ('admin', 'sub_admin', 'moderator')),
  capabilities text[] not null default '{}',
  status text not null default 'active' check (status in ('active', 'revoked', 'expired')),
  granted_by_user_id uuid references users(id) on delete set null,
  granted_by_kind text not null default 'superadmin_token',
  revoked_by_user_id uuid references users(id) on delete set null,
  reason text not null default '',
  starts_at timestamptz not null default now(),
  expires_at timestamptz,
  revoked_at timestamptz,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create unique index if not exists admin_role_assignments_active_unique
  on admin_role_assignments(user_id, role)
  where status = 'active' and revoked_at is null;

create index if not exists admin_role_assignments_user_status_idx
  on admin_role_assignments(user_id, status, updated_at desc);

create index if not exists admin_role_assignments_role_status_idx
  on admin_role_assignments(role, status, updated_at desc);

create table if not exists admin_audit_events (
  id uuid primary key,
  actor_kind text not null,
  actor_user_id uuid references users(id) on delete set null,
  action text not null,
  target_type text not null,
  target_user_id uuid references users(id) on delete set null,
  target_id uuid,
  capabilities text[] not null default '{}',
  summary text not null default '',
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now()
);

create index if not exists admin_audit_events_created_at_idx
  on admin_audit_events(created_at desc);

create index if not exists admin_audit_events_actor_idx
  on admin_audit_events(actor_kind, actor_user_id, created_at desc);

create index if not exists admin_audit_events_target_idx
  on admin_audit_events(target_type, target_id, created_at desc);

create index if not exists admin_audit_events_action_idx
  on admin_audit_events(action, created_at desc);
