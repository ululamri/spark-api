-- PASS 17B fix: keep superadmin as the legacy root role.
-- Database role assignments are delegated roles only: admin and moderator.
-- If an earlier migration produced a database superadmin assignment, convert it
-- back to delegated admin so the root path stays controlled by server config.

update admin_role_assignments
set role = 'admin',
    metadata = jsonb_set(
      jsonb_set(coalesce(metadata, '{}'::jsonb), '{legacy_root_boundary}', 'true'::jsonb, true),
      '{superadmin_db_role_removed_by}', '"pass_17b_legacy_root_alignment"'::jsonb, true
    ),
    updated_at = now()
where role = 'superadmin'
  and status = 'active'
  and revoked_at is null;
