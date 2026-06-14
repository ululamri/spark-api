-- PASS PUBLIC-SOCIAL-09B — Social release hardening
-- Keep the public social surface free from demo/default leakage and align
-- backend reaction behavior with the frontend single-reaction model.

create or replace function social_replace_existing_reaction()
returns trigger as $$
begin
  if new.post_id is not null then
    delete from social_reactions
    where user_id = new.user_id
      and post_id = new.post_id;
  end if;

  if new.comment_id is not null then
    delete from social_reactions
    where user_id = new.user_id
      and comment_id = new.comment_id;
  end if;

  return new;
end;
$$ language plpgsql;

drop trigger if exists social_reactions_replace_existing_before_insert on social_reactions;
create trigger social_reactions_replace_existing_before_insert
before insert on social_reactions
for each row
execute function social_replace_existing_reaction();

with ranked_post_reactions as (
  select id,
         row_number() over (
           partition by user_id, post_id
           order by updated_at desc, created_at desc, id desc
         ) as rn
  from social_reactions
  where post_id is not null
), ranked_comment_reactions as (
  select id,
         row_number() over (
           partition by user_id, comment_id
           order by updated_at desc, created_at desc, id desc
         ) as rn
  from social_reactions
  where comment_id is not null
), reaction_duplicates as (
  select id from ranked_post_reactions where rn > 1
  union all
  select id from ranked_comment_reactions where rn > 1
)
delete from social_reactions
where id in (select id from reaction_duplicates);

create or replace function profile_public_display_name_guard()
returns trigger as $$
declare
  email_local_part text;
begin
  select split_part(email, '@', 1)
    into email_local_part
  from users
  where id = new.user_id;

  if new.display_name is null
     or btrim(new.display_name) = ''
     or (email_local_part is not null and btrim(new.display_name) = email_local_part)
  then
    new.display_name := 'Pengguna Spark';
  end if;

  return new;
end;
$$ language plpgsql;

drop trigger if exists profiles_public_display_name_guard_before_write on profiles;
create trigger profiles_public_display_name_guard_before_write
before insert or update of display_name on profiles
for each row
execute function profile_public_display_name_guard();

insert into profiles (user_id, display_name)
select u.id, 'Pengguna Spark'
from users u
where u.status = 'active'
on conflict (user_id) do nothing;

update profiles p
set display_name = 'Pengguna Spark',
    updated_at = now()
from users u
where u.id = p.user_id
  and (
    p.display_name is null
    or btrim(p.display_name) = ''
    or btrim(p.display_name) = split_part(u.email, '@', 1)
  );

create or replace function ensure_generic_profile_for_new_user()
returns trigger as $$
begin
  if new.status = 'active' then
    insert into profiles (user_id, display_name)
    values (new.id, 'Pengguna Spark')
    on conflict (user_id) do nothing;
  end if;

  return new;
end;
$$ language plpgsql;

drop trigger if exists users_ensure_generic_profile_after_insert on users;
create trigger users_ensure_generic_profile_after_insert
after insert on users
for each row
execute function ensure_generic_profile_for_new_user();
