-- PASS 17B follow-up: canonicalize delegated admin role naming.
-- `sub_admin` was the early internal name. The production role is now `admin`.

update admin_role_assignments legacy
set status = 'revoked',
    revoked_at = coalesce(revoked_at, now()),
    reason = case
      when coalesce(reason, '') = '' then 'Auto-revoked duplicate legacy sub_admin role after admin role migration.'
      else reason
    end,
    updated_at = now()
where legacy.role = 'sub_admin'
  and legacy.status = 'active'
  and legacy.revoked_at is null
  and exists (
    select 1
    from admin_role_assignments current
    where current.user_id = legacy.user_id
      and current.role = 'admin'
      and current.status = 'active'
      and current.revoked_at is null
  );

update admin_role_assignments
set role = 'admin',
    metadata = jsonb_set(coalesce(metadata, '{}'::jsonb), '{migrated_from_role}', '"sub_admin"'::jsonb, true),
    updated_at = now()
where role = 'sub_admin'
  and status = 'active'
  and revoked_at is null;
