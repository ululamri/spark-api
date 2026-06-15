-- PASS ADMIN-RBAC-01 — Admin role and capability foundation
-- Adds super-admin bootstrap compatibility plus user-based sub-admin/moderator assignments.

create table if not exists admin_role_assignments (
  id uuid primary key,
  user_id uuid not null references users(id) on delete cascade,
  role text not null,
  capabilities text[] not null default '{}'::text[],
  status text not null default 'active',
  granted_by_user_id uuid references users(id) on delete set null,
  granted_by_kind text not null default 'super_admin_token',
  reason text not null default '',
  starts_at timestamptz not null default now(),
  expires_at timestamptz,
  revoked_at timestamptz,
  revoked_by_user_id uuid references users(id) on delete set null,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint admin_role_assignments_role_check check (role in ('sub_admin', 'moderator')),
  constraint admin_role_assignments_status_check check (status in ('active', 'revoked', 'expired')),
  constraint admin_role_assignments_granted_by_kind_check check (granted_by_kind in ('super_admin_token', 'super_admin_user', 'sub_admin')),
  constraint admin_role_assignments_time_check check (expires_at is null or expires_at > starts_at)
);

create unique index if not exists admin_role_assignments_one_active_role_idx
  on admin_role_assignments(user_id, role)
  where status = 'active' and revoked_at is null;

create index if not exists admin_role_assignments_user_idx
  on admin_role_assignments(user_id, status, created_at desc);

create index if not exists admin_role_assignments_capabilities_idx
  on admin_role_assignments using gin(capabilities);

create table if not exists admin_audit_events (
  id uuid primary key,
  actor_kind text not null,
  actor_user_id uuid references users(id) on delete set null,
  action text not null,
  target_type text not null,
  target_user_id uuid references users(id) on delete set null,
  target_id uuid,
  capabilities text[] not null default '{}'::text[],
  summary text not null default '',
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  constraint admin_audit_events_actor_kind_check check (actor_kind in ('super_admin_token', 'super_admin_user', 'sub_admin', 'moderator', 'system')),
  constraint admin_audit_events_target_type_check check (target_type in ('user', 'role_assignment', 'policy', 'moderation_action', 'ai_settings', 'system'))
);

create index if not exists admin_audit_events_actor_idx
  on admin_audit_events(actor_kind, actor_user_id, created_at desc);

create index if not exists admin_audit_events_target_idx
  on admin_audit_events(target_type, target_user_id, target_id, created_at desc);

insert into content_policy_settings (key, value, locked, description) values
  (
    'admin_rbac_capabilities',
    '{
      "super_admin":["developer_access","admin_manage","policy_manage","ai_manage","moderation_read","moderation_action","moderation_restore","user_safety_manage","reports_manage","content_read","media_review","audit_read"],
      "sub_admin_allowed":["policy_manage","ai_manage","moderation_read","moderation_action","moderation_restore","user_safety_manage","reports_manage","content_read","media_review","audit_read"],
      "moderator_allowed":["moderation_read","moderation_action","reports_manage","content_read","media_review","audit_read"]
    }'::jsonb,
    true,
    'Role capability catalog for super-admin, sub-admin, and moderator access.'
  ),
  (
    'admin_rbac_runtime',
    '{"version":"admin-rbac-01","super_admin_token_bootstrap":true,"session_based_sub_admin":true,"session_based_moderator":true}'::jsonb,
    false,
    'Admin RBAC runtime posture.'
  )
on conflict (key) do update set
  value = excluded.value,
  locked = excluded.locked,
  description = excluded.description,
  updated_at = now();
