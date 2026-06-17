-- PASS 17B follow-up: if there is exactly one active delegated admin assignment,
-- promote it to the canonical database-backed superadmin role.
-- This does not touch users, passwords, sessions, or the bootstrap KARYRA_ADMIN_TOKEN.

-- Existing environments may have the early role check without `superadmin`.
do $$
declare
  constraint_name text;
begin
  for constraint_name in
    select conname
    from pg_constraint
    where conrelid = 'admin_role_assignments'::regclass
      and contype = 'c'
      and pg_get_constraintdef(oid) like '%role%'
  loop
    execute format('alter table admin_role_assignments drop constraint if exists %I', constraint_name);
  end loop;

  alter table admin_role_assignments
    add constraint admin_role_assignments_role_check
    check (role in ('superadmin', 'admin', 'sub_admin', 'moderator'));
end $$;

with active_count as (
  select count(*)::int as total
  from admin_role_assignments
  where status = 'active'
    and revoked_at is null
), promoted as (
  update admin_role_assignments
  set role = 'superadmin',
      capabilities = array[
        'developer_access',
        'admin_manage',
        'policy_manage',
        'ai_manage',
        'ml_moderation_manage',
        'moderation_read',
        'moderation_action',
        'moderation_restore',
        'moderation_bulk',
        'user_safety_manage',
        'reports_manage',
        'content_read',
        'content_create',
        'content_edit',
        'content_publish',
        'content_archive',
        'media_review',
        'audit_read'
      ]::text[],
      reason = case
        when coalesce(reason, '') = '' then 'Auto-promoted as the only active admin assignment during PASS 17B superadmin bootstrap.'
        else reason
      end,
      metadata = jsonb_set(
        jsonb_set(coalesce(metadata, '{}'::jsonb), '{promoted_to_superadmin}', 'true'::jsonb, true),
        '{promoted_by}', '"pass_17b_single_active_admin_migration"'::jsonb, true
      ),
      updated_at = now()
  where status = 'active'
    and revoked_at is null
    and role in ('admin', 'sub_admin', 'moderator')
    and (select total from active_count) = 1
    and not exists (
      select 1
      from admin_role_assignments current
      where current.role = 'superadmin'
        and current.status = 'active'
        and current.revoked_at is null
    )
  returning id, user_id
)
insert into admin_audit_events (
  id, actor_kind, actor_user_id, action, target_type, target_user_id,
  target_id, capabilities, summary, metadata
)
select gen_random_uuid(),
       'system_migration',
       null,
       'admin_role_auto_promote_superadmin',
       'user',
       user_id,
       id,
       array['developer_access', 'admin_manage', 'audit_read']::text[],
       'Single active admin assignment was promoted to superadmin.',
       jsonb_build_object('migration', '202606170003_promote_single_active_admin_to_superadmin')
from promoted;
