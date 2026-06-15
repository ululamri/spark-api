-- PASS AUDIT-HOTFIX-01 — social/comment/media runtime regression fixes
-- Feed comment hydration is restored in Rust code. This migration removes the
-- DB-side upload-intent rate-limit trigger that could turn media uploads into
-- opaque internal errors on fresh sessions/devices. Social write rate limits
-- remain enforced by the Rust moderation engine.

drop trigger if exists media_assets_policy_rate_limit_before_insert on media_assets;
drop function if exists content_policy_media_upload_rate_limit();

insert into content_policy_settings (key, value, locked, description) values
  (
    'media_upload_rate_limit_runtime',
    '{"status":"softened","reason":"moved away from DB trigger after audit hotfix","follow_up":"reintroduce in Rust handler/admin settings after runtime verification"}'::jsonb,
    false,
    'Media upload rate limit runtime posture after audit hotfix.'
  )
on conflict (key) do update set
  value = excluded.value,
  locked = excluded.locked,
  description = excluded.description,
  updated_at = now();
