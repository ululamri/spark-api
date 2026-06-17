-- PASS 17E-fix: Relax legacy admin audit check constraints for production admin modules.
-- The audit table is append-only operational evidence. Older live schemas had narrow
-- enum-like checks that rejected newer CMS, social moderation, bulk moderation, and
-- future ML moderation audit events.

alter table admin_audit_events
  drop constraint if exists admin_audit_events_actor_kind_check,
  drop constraint if exists admin_audit_events_action_check,
  drop constraint if exists admin_audit_events_target_type_check;

alter table admin_audit_events
  add constraint admin_audit_events_actor_kind_non_empty
    check (length(trim(actor_kind)) > 0),
  add constraint admin_audit_events_action_non_empty
    check (length(trim(action)) > 0),
  add constraint admin_audit_events_target_type_non_empty
    check (length(trim(target_type)) > 0);
