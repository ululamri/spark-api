-- PASS 17B-2: enforce the final role boundary.
-- Superadmin remains the legacy root path. Database assignments are only admin/moderator.

update admin_role_assignments
set role = 'admin',
    metadata = jsonb_set(
      coalesce(metadata, '{}'::jsonb),
      '{delegated_role_boundary_enforced}',
      'true'::jsonb,
      true
    ),
    updated_at = now()
where role = 'superadmin';

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
    check (role in ('admin', 'sub_admin', 'moderator'));
end $$;
