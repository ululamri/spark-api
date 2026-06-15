-- PASS CONTENT-POLICY-02-04 — Runtime support for rules, limits, and media quarantine
-- Rust enforces rule moderation and adaptive limits for social writes.
-- This migration adds database-side media safeguards so public assets can be
-- quarantined before they are linked to profile/social surfaces.

create extension if not exists pgcrypto;

create or replace function content_policy_text_has_any(input text, needles text[])
returns boolean as $$
declare
  needle text;
begin
  if input is null then
    return false;
  end if;

  foreach needle in array needles loop
    if position(needle in input) > 0 then
      return true;
    end if;
  end loop;

  return false;
end;
$$ language plpgsql immutable;

create or replace function content_policy_media_upload_rate_limit()
returns trigger as $$
declare
  purpose text;
  scope_name text;
  limit_count integer;
  used integer;
  window_start timestamptz;
begin
  purpose := coalesce(new.metadata->>'purpose', 'general');
  scope_name := case when purpose = 'avatar' then 'avatar_upload' else 'media_upload' end;
  limit_count := case when scope_name = 'avatar_upload' then 5 else 20 end;
  window_start := date_trunc('day', now());

  insert into user_rate_limit_buckets (
    user_id, scope, window_start, window_seconds, used_count, limit_count, reset_at
  ) values (
    new.owner_user_id, scope_name, window_start, 86400, 1, limit_count, window_start + interval '1 day'
  )
  on conflict (user_id, scope, window_start) do update set
    used_count = user_rate_limit_buckets.used_count + 1,
    limit_count = excluded.limit_count,
    reset_at = excluded.reset_at,
    updated_at = now()
  returning used_count into used;

  if used > limit_count then
    raise exception 'media upload rate limit exceeded for %', scope_name using errcode = 'P0001';
  end if;

  return new;
end;
$$ language plpgsql;

drop trigger if exists media_assets_policy_rate_limit_before_insert on media_assets;
create trigger media_assets_policy_rate_limit_before_insert
before insert on media_assets
for each row
execute function content_policy_media_upload_rate_limit();

create or replace function content_policy_media_moderation_before_uploaded()
returns trigger as $$
declare
  scan_text text;
  categories text[] := '{}'::text[];
  decision text := 'allow';
  moderation_state text := 'allowed';
  severity text := 'low';
  score numeric(6,5) := 0;
  message text := '';
  event_id uuid;
  total_points integer := 0;
begin
  if not (new.status = 'uploaded' and old.status is distinct from 'uploaded') then
    return new;
  end if;

  scan_text := lower(
    coalesce(new.original_file_name, '') || ' ' ||
    coalesce(new.mime_type, '') || ' ' ||
    coalesce(new.metadata::text, '')
  );

  if new.mime_type like 'image/%' and content_policy_text_has_any(scan_text, array['nsfw','adult-content','explicit-image','unsafe-image']) then
    categories := array_append(categories, 'nsfw_explicit');
    decision := 'block';
    moderation_state := 'blocked';
    severity := 'high';
    score := greatest(score, 0.88000);
  end if;

  if content_policy_text_has_any(scan_text, array['slot','casino','judi','togel','betting','taruhan','parlay','jackpot','maxwin','scatter']) then
    categories := array_append(categories, 'gambling');
    decision := 'block';
    moderation_state := 'blocked';
    severity := 'high';
    score := greatest(score, 0.90000);
  end if;

  if content_policy_text_has_any(scan_text, array['sinyal trading','signal trading','pump group','grup pump','profit guarantee','jaminan profit','cuan harian','join vip','copy trade','copytrade']) then
    if decision <> 'block' then
      decision := 'review';
      moderation_state := 'pending_review';
      severity := 'high';
    end if;
    categories := array_append(categories, 'trading_solicitation');
    score := greatest(score, 0.78000);
  end if;

  if content_policy_text_has_any(scan_text, array['kode referral','referral exchange','paid promote','wa.me/','t.me/','bit.ly/','tinyurl.com/']) then
    if decision <> 'block' then
      decision := 'review';
      moderation_state := 'pending_review';
      severity := 'medium';
    end if;
    categories := array_append(categories, 'advertising');
    score := greatest(score, 0.65000);
  end if;

  new.moderation_decision := decision;
  new.moderation_status := moderation_state;
  new.moderation_categories := categories;
  new.moderation_score := score;
  new.moderation_checked_at := now();
  new.moderation_source := 'rules';

  if decision = 'allow' then
    new.moderation_message := '';
    return new;
  end if;

  message := case
    when decision = 'block' then 'Media could not be made public because it appears to violate the community safety policy.'
    else 'Media was saved for review before it can appear publicly.'
  end;

  new.moderation_message := message;
  new.visibility := 'private';
  new.public_url := null;

  event_id := gen_random_uuid();
  insert into moderation_events (
    id, actor_user_id, target_type, target_id, target_owner_user_id,
    event_type, decision, categories, severity, score, source,
    user_message, admin_summary, metadata
  ) values (
    event_id, new.owner_user_id, 'media', new.id, new.owner_user_id,
    'media_scan', decision, categories, severity, score, 'rules',
    message, 'Rule-based media metadata moderation matched policy indicators.',
    jsonb_build_object('object_key', new.object_key, 'mime_type', new.mime_type)
  );

  select coalesce(sum(strike_points), 0)::integer into total_points
  from content_policy_categories
  where key = any(categories);

  if total_points > 0 then
    insert into moderation_strikes (
      id, user_id, moderation_event_id, category, points, reason, source,
      decays_at, expires_at, metadata
    ) values (
      gen_random_uuid(), new.owner_user_id, event_id, array_to_string(categories, ','),
      total_points, message, 'rules', now() + interval '24 hours', now() + interval '30 days',
      jsonb_build_object('decision', decision, 'source', 'media_trigger')
    );

    insert into user_safety_scores (
      user_id, active_points, lifetime_points, safety_level, restriction_level,
      last_violation_at, next_decay_at
    ) values (
      new.owner_user_id, total_points, total_points, 'gentle_friction', 'gentle_friction',
      now(), now() + interval '24 hours'
    )
    on conflict (user_id) do update set
      active_points = user_safety_scores.active_points + excluded.active_points,
      lifetime_points = user_safety_scores.lifetime_points + excluded.lifetime_points,
      safety_level = case
        when user_safety_scores.active_points + excluded.active_points >= 15 then 'admin_escalation'
        when user_safety_scores.active_points + excluded.active_points >= 10 then 'temporary_restriction'
        when user_safety_scores.active_points + excluded.active_points >= 6 then 'probation'
        when user_safety_scores.active_points + excluded.active_points >= 3 then 'reduced_velocity'
        when user_safety_scores.active_points + excluded.active_points >= 1 then 'gentle_friction'
        else 'normal'
      end,
      restriction_level = case
        when user_safety_scores.active_points + excluded.active_points >= 15 then 'hard_block'
        when user_safety_scores.active_points + excluded.active_points >= 10 then 'temporary_restriction'
        when user_safety_scores.active_points + excluded.active_points >= 6 then 'probation'
        when user_safety_scores.active_points + excluded.active_points >= 3 then 'reduced_velocity'
        when user_safety_scores.active_points + excluded.active_points >= 1 then 'gentle_friction'
        else 'none'
      end,
      last_violation_at = now(),
      next_decay_at = coalesce(user_safety_scores.next_decay_at, now() + interval '24 hours'),
      updated_at = now();
  end if;

  return new;
end;
$$ language plpgsql;

drop trigger if exists media_assets_policy_moderation_before_uploaded on media_assets;
create trigger media_assets_policy_moderation_before_uploaded
before update of status on media_assets
for each row
execute function content_policy_media_moderation_before_uploaded();
