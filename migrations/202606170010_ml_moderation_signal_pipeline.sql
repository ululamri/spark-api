-- PASS 17F: ML moderation signal pipeline.
-- Human-in-the-loop signal store for social posts/comments. Signals never delete,
-- hide, remove, or restore content by themselves; they only provide review evidence.

create table if not exists social_moderation_ml_signals (
  id uuid primary key,
  target_type text not null check (target_type in ('post', 'comment')),
  target_id uuid not null,
  target_owner_user_id uuid references users(id) on delete set null,
  source text not null default 'combined' check (source in ('rules', 'local_ai', 'external_ai', 'combined')),
  status text not null check (status in ('clean', 'needs_review', 'high_risk', 'blocked_pending_review')),
  decision text not null check (decision in ('allow', 'review', 'block')),
  categories text[] not null default '{}',
  severity text not null default 'low' check (severity in ('low', 'medium', 'high', 'critical')),
  score numeric(6,5) not null default 0,
  summary text not null default '',
  recommendation text not null default '',
  moderation_event_id uuid references moderation_events(id) on delete set null,
  model_run_ids uuid[] not null default '{}',
  created_by_kind text not null,
  created_by_user_id uuid references users(id) on delete set null,
  reviewed_by_user_id uuid references users(id) on delete set null,
  reviewed_at timestamptz,
  review_note text not null default '',
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists social_moderation_ml_signals_target_idx
  on social_moderation_ml_signals(target_type, target_id, created_at desc);

create index if not exists social_moderation_ml_signals_status_idx
  on social_moderation_ml_signals(status, created_at desc);

create index if not exists social_moderation_ml_signals_owner_idx
  on social_moderation_ml_signals(target_owner_user_id, created_at desc);

create index if not exists social_moderation_ml_signals_review_idx
  on social_moderation_ml_signals(reviewed_at, created_at desc);
