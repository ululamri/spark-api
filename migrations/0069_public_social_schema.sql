-- PASS PUBLIC-SOCIAL-01 — Public Social Schema
-- Source of truth for API-backed community feed, author profile cards,
-- media attachments through media_links, and minimal moderation.

create table if not exists social_posts (
  id uuid primary key,
  author_user_id uuid not null references users(id) on delete cascade,
  kind text not null default 'post',
  body text not null default '',
  visibility text not null default 'community',
  status text not null default 'published',
  published_at timestamptz not null default now(),
  hidden_at timestamptz,
  removed_at timestamptz,
  deleted_at timestamptz,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint social_posts_kind_check check (
    kind in ('post', 'reflection', 'question', 'proof', 'milestone', 'update')
  ),
  constraint social_posts_visibility_check check (
    visibility in ('public', 'community', 'followers', 'private')
  ),
  constraint social_posts_status_check check (
    status in ('published', 'hidden', 'removed', 'deleted')
  ),
  constraint social_posts_body_length_check check (char_length(body) <= 4000)
);

create index if not exists social_posts_feed_idx
  on social_posts(status, visibility, published_at desc, id desc)
  where status = 'published' and visibility in ('public', 'community', 'followers');

create index if not exists social_posts_author_idx
  on social_posts(author_user_id, status, published_at desc);

create index if not exists social_posts_kind_idx
  on social_posts(kind, status, published_at desc);

create table if not exists social_comments (
  id uuid primary key,
  post_id uuid not null references social_posts(id) on delete cascade,
  author_user_id uuid not null references users(id) on delete cascade,
  parent_comment_id uuid references social_comments(id) on delete cascade,
  body text not null,
  status text not null default 'published',
  hidden_at timestamptz,
  removed_at timestamptz,
  deleted_at timestamptz,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint social_comments_status_check check (
    status in ('published', 'hidden', 'removed', 'deleted')
  ),
  constraint social_comments_body_length_check check (
    char_length(body) > 0 and char_length(body) <= 2000
  )
);

create index if not exists social_comments_post_idx
  on social_comments(post_id, status, created_at asc, id asc);

create index if not exists social_comments_author_idx
  on social_comments(author_user_id, status, created_at desc);

create table if not exists social_reactions (
  id uuid primary key,
  user_id uuid not null references users(id) on delete cascade,
  post_id uuid references social_posts(id) on delete cascade,
  comment_id uuid references social_comments(id) on delete cascade,
  kind text not null default 'like',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint social_reactions_target_check check (
    (post_id is not null and comment_id is null)
    or (post_id is null and comment_id is not null)
  ),
  constraint social_reactions_kind_check check (
    kind in ('like', 'support', 'insightful', 'celebrate')
  )
);

create unique index if not exists social_reactions_user_post_kind_idx
  on social_reactions(user_id, post_id, kind)
  where post_id is not null;

create unique index if not exists social_reactions_user_comment_kind_idx
  on social_reactions(user_id, comment_id, kind)
  where comment_id is not null;

create index if not exists social_reactions_post_idx
  on social_reactions(post_id, kind, created_at desc)
  where post_id is not null;

create index if not exists social_reactions_comment_idx
  on social_reactions(comment_id, kind, created_at desc)
  where comment_id is not null;

create table if not exists social_follows (
  follower_user_id uuid not null references users(id) on delete cascade,
  followed_user_id uuid not null references users(id) on delete cascade,
  status text not null default 'following',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (follower_user_id, followed_user_id),
  constraint social_follows_no_self_check check (follower_user_id <> followed_user_id),
  constraint social_follows_status_check check (status in ('following', 'muted', 'blocked'))
);

create index if not exists social_follows_followed_idx
  on social_follows(followed_user_id, status, updated_at desc);

create table if not exists social_post_hides (
  user_id uuid not null references users(id) on delete cascade,
  post_id uuid not null references social_posts(id) on delete cascade,
  reason text not null default 'viewer_hidden',
  created_at timestamptz not null default now(),
  primary key (user_id, post_id),
  constraint social_post_hides_reason_check check (
    reason in ('viewer_hidden', 'not_relevant', 'already_seen')
  )
);

create index if not exists social_post_hides_user_idx
  on social_post_hides(user_id, created_at desc);

create table if not exists social_reports (
  id uuid primary key,
  reporter_user_id uuid not null references users(id) on delete cascade,
  target_type text not null,
  target_id uuid not null,
  reason text not null,
  details text not null default '',
  status text not null default 'pending',
  reviewed_by_user_id uuid references users(id) on delete set null,
  reviewed_at timestamptz,
  action_id uuid,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint social_reports_target_type_check check (
    target_type in ('post', 'comment', 'profile', 'media')
  ),
  constraint social_reports_reason_check check (
    reason in ('spam', 'abuse', 'harassment', 'unsafe', 'privacy', 'misleading', 'other')
  ),
  constraint social_reports_status_check check (
    status in ('pending', 'reviewed', 'actioned', 'dismissed')
  ),
  constraint social_reports_details_length_check check (char_length(details) <= 2000)
);

create index if not exists social_reports_queue_idx
  on social_reports(status, created_at asc);

create index if not exists social_reports_target_idx
  on social_reports(target_type, target_id, status, created_at desc);

create unique index if not exists social_reports_reporter_target_reason_idx
  on social_reports(reporter_user_id, target_type, target_id, reason)
  where status = 'pending';

create table if not exists social_moderation_actions (
  id uuid primary key,
  moderator_user_id uuid references users(id) on delete set null,
  target_type text not null,
  target_id uuid not null,
  action text not null,
  reason text not null default '',
  payload jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  constraint social_moderation_target_type_check check (
    target_type in ('post', 'comment', 'profile', 'media', 'report')
  ),
  constraint social_moderation_action_check check (
    action in ('hide', 'remove', 'restore', 'dismiss_report', 'mark_reviewed')
  ),
  constraint social_moderation_reason_length_check check (char_length(reason) <= 1000)
);

alter table social_reports
  drop constraint if exists social_reports_action_id_fkey;

alter table social_reports
  add constraint social_reports_action_id_fkey
  foreign key (action_id) references social_moderation_actions(id) on delete set null;

create index if not exists social_moderation_actions_target_idx
  on social_moderation_actions(target_type, target_id, created_at desc);

create index if not exists social_moderation_actions_moderator_idx
  on social_moderation_actions(moderator_user_id, created_at desc);

create unique index if not exists profiles_handle_lower_unique_idx
  on profiles(lower(handle))
  where handle is not null and handle <> '';

create index if not exists profiles_social_visibility_idx
  on profiles(visibility, updated_at desc)
  where visibility in ('public', 'community');

create index if not exists media_links_entity_lookup_idx
  on media_links(entity_type, entity_id, purpose, created_at desc);

create index if not exists media_assets_social_public_idx
  on media_assets(owner_user_id, status, visibility, created_at desc)
  where status = 'uploaded' and visibility = 'public';
