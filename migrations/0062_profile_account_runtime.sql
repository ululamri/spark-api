-- Pass 62 — Profile Account Runtime
-- Adds user-facing profile fields used by the Spark frontend account/profile surface.

alter table profiles
  add column if not exists bio text not null default '',
  add column if not exists location text not null default '',
  add column if not exists visibility text not null default 'community',
  add column if not exists avatar_preset text not null default 'spark',
  add column if not exists avatar_url text;

update profiles
set visibility = 'community'
where visibility is null or visibility = '';

update profiles
set avatar_preset = 'spark'
where avatar_preset is null or avatar_preset = '';

alter table profiles
  drop constraint if exists profiles_visibility_check;

alter table profiles
  add constraint profiles_visibility_check
  check (visibility in ('private', 'community', 'public'));

alter table profiles
  drop constraint if exists profiles_avatar_preset_check;

alter table profiles
  add constraint profiles_avatar_preset_check
  check (avatar_preset in ('spark', 'trophy', 'coffee', 'explorer', 'mentor'));
