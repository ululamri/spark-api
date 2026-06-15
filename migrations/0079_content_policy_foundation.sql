-- PASS CONTENT-POLICY-01 — Content policy and moderation foundation
-- Adds policy taxonomy, moderation audit tables, AI/settings foundations,
-- user safety score/restriction primitives, rate-limit buckets, and non-breaking
-- moderation status columns for social and media surfaces.

alter table social_posts
  add column if not exists moderation_status text not null default 'allowed',
  add column if not exists moderation_decision text,
  add column if not exists moderation_categories text[] not null default '{}'::text[],
  add column if not exists moderation_score numeric(6,5),
  add column if not exists moderation_checked_at timestamptz,
  add column if not exists moderation_source text,
  add column if not exists moderation_message text;

alter table social_comments
  add column if not exists moderation_status text not null default 'allowed',
  add column if not exists moderation_decision text,
  add column if not exists moderation_categories text[] not null default '{}'::text[],
  add column if not exists moderation_score numeric(6,5),
  add column if not exists moderation_checked_at timestamptz,
  add column if not exists moderation_source text,
  add column if not exists moderation_message text;

alter table media_assets
  add column if not exists moderation_status text not null default 'allowed',
  add column if not exists moderation_decision text,
  add column if not exists moderation_categories text[] not null default '{}'::text[],
  add column if not exists moderation_score numeric(6,5),
  add column if not exists moderation_checked_at timestamptz,
  add column if not exists moderation_source text,
  add column if not exists moderation_message text;

alter table social_posts
  drop constraint if exists social_posts_moderation_status_check,
  add constraint social_posts_moderation_status_check check (moderation_status in ('unreviewed', 'allowed', 'pending_review', 'blocked', 'admin_hidden', 'admin_removed', 'restored')),
  drop constraint if exists social_posts_moderation_decision_check,
  add constraint social_posts_moderation_decision_check check (moderation_decision is null or moderation_decision in ('allow', 'review', 'block', 'restrict')),
  drop constraint if exists social_posts_moderation_score_check,
  add constraint social_posts_moderation_score_check check (moderation_score is null or (moderation_score >= 0 and moderation_score <= 1));

alter table social_comments
  drop constraint if exists social_comments_moderation_status_check,
  add constraint social_comments_moderation_status_check check (moderation_status in ('unreviewed', 'allowed', 'pending_review', 'blocked', 'admin_hidden', 'admin_removed', 'restored')),
  drop constraint if exists social_comments_moderation_decision_check,
  add constraint social_comments_moderation_decision_check check (moderation_decision is null or moderation_decision in ('allow', 'review', 'block', 'restrict')),
  drop constraint if exists social_comments_moderation_score_check,
  add constraint social_comments_moderation_score_check check (moderation_score is null or (moderation_score >= 0 and moderation_score <= 1));

alter table media_assets
  drop constraint if exists media_assets_moderation_status_check,
  add constraint media_assets_moderation_status_check check (moderation_status in ('unreviewed', 'allowed', 'pending_review', 'blocked', 'admin_hidden', 'admin_removed', 'restored')),
  drop constraint if exists media_assets_moderation_decision_check,
  add constraint media_assets_moderation_decision_check check (moderation_decision is null or moderation_decision in ('allow', 'review', 'block', 'restrict')),
  drop constraint if exists media_assets_moderation_score_check,
  add constraint media_assets_moderation_score_check check (moderation_score is null or (moderation_score >= 0 and moderation_score <= 1));

create table if not exists content_policy_categories (
  key text primary key,
  label text not null,
  severity text not null default 'medium',
  default_decision text not null default 'review',
  strike_points integer not null default 1,
  locked boolean not null default false,
  description text not null default '',
  public_guidance text not null default '',
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint content_policy_categories_severity_check check (severity in ('low', 'medium', 'high', 'critical')),
  constraint content_policy_categories_decision_check check (default_decision in ('allow', 'review', 'block', 'restrict')),
  constraint content_policy_categories_strike_points_check check (strike_points >= 0 and strike_points <= 20)
);

create table if not exists content_policy_settings (
  key text primary key,
  value jsonb not null default '{}'::jsonb,
  locked boolean not null default false,
  description text not null default '',
  updated_by_user_id uuid references users(id) on delete set null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists moderation_events (
  id uuid primary key,
  actor_user_id uuid references users(id) on delete set null,
  target_type text not null,
  target_id uuid,
  target_owner_user_id uuid references users(id) on delete set null,
  event_type text not null,
  decision text not null,
  categories text[] not null default '{}'::text[],
  severity text not null default 'medium',
  score numeric(6,5),
  source text not null,
  user_message text not null default '',
  admin_summary text not null default '',
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  constraint moderation_events_target_type_check check (target_type in ('post', 'comment', 'profile', 'media', 'avatar', 'report', 'user', 'assistant_message', 'system')),
  constraint moderation_events_event_type_check check (event_type in ('pre_publish_scan', 'post_publish_scan', 'media_scan', 'user_report', 'admin_action', 'restriction', 'rate_limit', 'ai_review', 'appeal')),
  constraint moderation_events_decision_check check (decision in ('allow', 'review', 'block', 'restrict')),
  constraint moderation_events_severity_check check (severity in ('low', 'medium', 'high', 'critical')),
  constraint moderation_events_score_check check (score is null or (score >= 0 and score <= 1)),
  constraint moderation_events_source_check check (source in ('rules', 'local_ai', 'external_api', 'admin', 'user_report', 'system', 'combined'))
);

create index if not exists moderation_events_target_idx on moderation_events(target_type, target_id, created_at desc);
create index if not exists moderation_events_actor_idx on moderation_events(actor_user_id, created_at desc) where actor_user_id is not null;
create index if not exists moderation_events_owner_idx on moderation_events(target_owner_user_id, created_at desc) where target_owner_user_id is not null;
create index if not exists moderation_events_queue_idx on moderation_events(decision, severity, created_at asc) where decision in ('review', 'block', 'restrict');

create table if not exists moderation_model_runs (
  id uuid primary key,
  moderation_event_id uuid references moderation_events(id) on delete cascade,
  provider text not null,
  model text not null,
  input_type text not null,
  decision text not null,
  categories text[] not null default '{}'::text[],
  score numeric(6,5),
  latency_ms integer,
  prompt_tokens integer,
  completion_tokens integer,
  raw_response jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  constraint moderation_model_runs_input_type_check check (input_type in ('text', 'image', 'multimodal', 'metadata')),
  constraint moderation_model_runs_decision_check check (decision in ('allow', 'review', 'block', 'restrict')),
  constraint moderation_model_runs_score_check check (score is null or (score >= 0 and score <= 1)),
  constraint moderation_model_runs_latency_check check (latency_ms is null or latency_ms >= 0),
  constraint moderation_model_runs_tokens_check check ((prompt_tokens is null or prompt_tokens >= 0) and (completion_tokens is null or completion_tokens >= 0))
);

create index if not exists moderation_model_runs_event_idx on moderation_model_runs(moderation_event_id, created_at desc);
create index if not exists moderation_model_runs_provider_idx on moderation_model_runs(provider, model, created_at desc);

create table if not exists user_safety_scores (
  user_id uuid primary key references users(id) on delete cascade,
  active_points integer not null default 0,
  lifetime_points integer not null default 0,
  safety_level text not null default 'normal',
  restriction_level text not null default 'none',
  last_violation_at timestamptz,
  next_decay_at timestamptz,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint user_safety_scores_points_check check (active_points >= 0 and lifetime_points >= 0),
  constraint user_safety_scores_level_check check (safety_level in ('normal', 'gentle_friction', 'reduced_velocity', 'probation', 'temporary_restriction', 'admin_escalation')),
  constraint user_safety_scores_restriction_level_check check (restriction_level in ('none', 'gentle_friction', 'reduced_velocity', 'probation', 'temporary_restriction', 'hard_block'))
);

create table if not exists moderation_strikes (
  id uuid primary key,
  user_id uuid not null references users(id) on delete cascade,
  moderation_event_id uuid references moderation_events(id) on delete set null,
  category text not null,
  points integer not null,
  reason text not null default '',
  source text not null,
  decays_at timestamptz,
  expires_at timestamptz,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  constraint moderation_strikes_points_check check (points > 0 and points <= 20),
  constraint moderation_strikes_source_check check (source in ('rules', 'local_ai', 'external_api', 'admin', 'system', 'combined'))
);

create index if not exists moderation_strikes_user_idx on moderation_strikes(user_id, expires_at, created_at desc);
create index if not exists moderation_strikes_decay_idx on moderation_strikes(decays_at asc) where decays_at is not null;

create table if not exists user_restrictions (
  id uuid primary key,
  user_id uuid not null references users(id) on delete cascade,
  scope text not null,
  level text not null,
  reason text not null default '',
  source_event_id uuid references moderation_events(id) on delete set null,
  created_by_user_id uuid references users(id) on delete set null,
  starts_at timestamptz not null default now(),
  expires_at timestamptz,
  lifted_at timestamptz,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint user_restrictions_scope_check check (scope in ('all', 'social_post_create', 'social_comment_create', 'social_reaction_create', 'social_report_create', 'media_upload', 'avatar_upload', 'profile_update', 'follow_user', 'ai_user_chat', 'admin_review')),
  constraint user_restrictions_level_check check (level in ('gentle_friction', 'reduced_velocity', 'probation', 'temporary_restriction', 'hard_block')),
  constraint user_restrictions_time_check check (expires_at is null or expires_at > starts_at)
);

create index if not exists user_restrictions_user_active_idx on user_restrictions(user_id, scope, level, expires_at) where lifted_at is null;

create table if not exists user_rate_limit_buckets (
  user_id uuid not null references users(id) on delete cascade,
  scope text not null,
  window_start timestamptz not null,
  window_seconds integer not null,
  used_count integer not null default 0,
  limit_count integer not null,
  reset_at timestamptz not null,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (user_id, scope, window_start),
  constraint user_rate_limit_buckets_scope_check check (scope in ('social_post_create', 'social_comment_create', 'social_reaction_create', 'social_report_create', 'media_upload', 'avatar_upload', 'profile_update', 'follow_user', 'ai_user_chat', 'admin_ai_review')),
  constraint user_rate_limit_buckets_counts_check check (used_count >= 0 and limit_count >= 0 and window_seconds > 0)
);

create index if not exists user_rate_limit_buckets_reset_idx on user_rate_limit_buckets(reset_at asc);

create table if not exists ai_provider_settings (
  provider text primary key,
  enabled boolean not null default false,
  mode text not null default 'admin_only',
  base_url text,
  model text,
  priority integer not null default 100,
  timeout_ms integer not null default 30000,
  metadata jsonb not null default '{}'::jsonb,
  updated_by_user_id uuid references users(id) on delete set null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint ai_provider_settings_mode_check check (mode in ('user_assistant', 'admin_only', 'moderation_only', 'disabled')),
  constraint ai_provider_settings_timeout_check check (timeout_ms between 1000 and 120000)
);

create table if not exists ai_assistant_settings (
  key text primary key,
  enabled boolean not null default false,
  value jsonb not null default '{}'::jsonb,
  locked boolean not null default false,
  description text not null default '',
  updated_by_user_id uuid references users(id) on delete set null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists moderation_appeals (
  id uuid primary key,
  user_id uuid not null references users(id) on delete cascade,
  target_type text not null,
  target_id uuid not null,
  moderation_event_id uuid references moderation_events(id) on delete set null,
  status text not null default 'pending',
  message text not null default '',
  reviewed_by_user_id uuid references users(id) on delete set null,
  reviewed_at timestamptz,
  resolution text not null default '',
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint moderation_appeals_target_type_check check (target_type in ('post', 'comment', 'profile', 'media', 'avatar', 'user')),
  constraint moderation_appeals_status_check check (status in ('pending', 'reviewed', 'accepted', 'rejected', 'cancelled')),
  constraint moderation_appeals_message_length_check check (char_length(message) <= 2000),
  constraint moderation_appeals_resolution_length_check check (char_length(resolution) <= 2000)
);

create index if not exists moderation_appeals_queue_idx on moderation_appeals(status, created_at asc);
create index if not exists moderation_appeals_user_idx on moderation_appeals(user_id, created_at desc);

insert into content_policy_categories (key, label, severity, default_decision, strike_points, locked, description, public_guidance) values
  ('sexual_minors', 'Sexual exploitation of minors', 'critical', 'block', 10, true, 'Sexual or exploitative content involving minors is not permitted.', 'This category is always blocked and escalated for safety review.'),
  ('nsfw_explicit', 'Explicit sexual content', 'high', 'block', 5, false, 'Explicit sexual images, nudity, or sexual solicitation are not allowed in the learning community.', 'Keep shared media safe for a public learning environment.'),
  ('gambling', 'Gambling and betting', 'high', 'block', 4, false, 'Casino, slot, betting, togel, and similar gambling promotion are not allowed.', 'Do not use Karyra Spark to promote gambling or betting.'),
  ('trading_solicitation', 'Trading solicitation', 'high', 'review', 3, false, 'Trading signals, profit guarantees, pump groups, exchange referral spam, and investment solicitation are restricted.', 'Educational blockchain discussion is allowed; trading solicitation is not.'),
  ('financial_scam', 'Financial scam', 'critical', 'block', 8, true, 'Fraudulent financial schemes, fake airdrops, wallet drain attempts, and deceptive offers are not permitted.', 'Never request funds, seed phrases, private keys, or deceptive wallet actions.'),
  ('phishing', 'Phishing', 'critical', 'block', 8, true, 'Attempts to steal credentials, session tokens, wallet secrets, or recovery phrases are not permitted.', 'Account and wallet safety is protected by default.'),
  ('private_key_request', 'Private key or seed phrase request', 'critical', 'block', 8, true, 'Requests for private keys, seed phrases, or wallet recovery data are not permitted.', 'No user should ever share wallet secrets.'),
  ('malware', 'Malware or abuse tooling', 'critical', 'block', 10, true, 'Malware, wallet drainers, credential theft, and abuse tooling are not permitted.', 'Security education is allowed; abuse instructions are not.'),
  ('spam', 'Spam', 'medium', 'review', 2, false, 'Repeated, low-value, automated, or disruptive content is restricted.', 'Keep community posts relevant and useful.'),
  ('advertising', 'Advertising', 'medium', 'review', 2, false, 'Unsolicited ads, aggressive promotion, and unrelated commercial posts are restricted.', 'Promotional content may be limited or removed.'),
  ('referral_abuse', 'Referral abuse', 'medium', 'review', 2, false, 'Referral farming, exchange invite spam, and incentive abuse are restricted.', 'Do not use the community primarily for referrals.'),
  ('harassment', 'Harassment', 'high', 'review', 3, false, 'Targeted harassment, abusive language, bullying, and intimidation are restricted.', 'Respectful disagreement is allowed; harassment is not.'),
  ('hate', 'Hate or dehumanization', 'critical', 'block', 6, true, 'Hate, dehumanization, or attacks against protected characteristics are not permitted.', 'Karyra Spark is designed for inclusive learning.'),
  ('violence_threat', 'Credible threat of violence', 'critical', 'block', 10, true, 'Credible threats, incitement, or instructions for violence are not permitted.', 'Threatening content is blocked and escalated.'),
  ('doxxing', 'Doxxing and personal data exposure', 'critical', 'block', 8, true, 'Sharing another person’s private data without consent is not permitted.', 'Protect personal privacy and safety.'),
  ('illegal_goods', 'Illegal goods or services', 'high', 'block', 5, false, 'Illegal goods, drugs, weapons, forged documents, and similar services are not permitted.', 'Do not use Karyra Spark to facilitate illegal transactions.'),
  ('external_link_risk', 'Risky external link', 'medium', 'review', 1, false, 'Suspicious links may be reviewed before publication.', 'Links should be relevant, safe, and transparent.'),
  ('toxicity_low', 'Low-level toxicity', 'low', 'review', 1, false, 'Mild insults or disruptive tone may be slowed, reviewed, or warned.', 'Keep feedback constructive and human.')
on conflict (key) do update set
  label = excluded.label,
  severity = excluded.severity,
  default_decision = excluded.default_decision,
  strike_points = excluded.strike_points,
  locked = excluded.locked,
  description = excluded.description,
  public_guidance = excluded.public_guidance,
  updated_at = now();

insert into content_policy_settings (key, value, locked, description) values
  ('policy_version', '{"version":"content-policy-01","status":"foundation"}'::jsonb, true, 'Current content policy foundation version.'),
  ('hard_safety_baseline', '{"categories":["sexual_minors","financial_scam","phishing","private_key_request","malware","hate","violence_threat","doxxing"],"mutable":false}'::jsonb, true, 'Categories that cannot be disabled from the admin UI.'),
  ('default_decision_thresholds', '{"review_score":0.55,"block_score":0.85,"uncertain_score":0.75}'::jsonb, false, 'Default decision thresholds for rules, local AI, and external review providers.'),
  ('strike_ladder', '{"gentle_friction":1,"reduced_velocity":3,"probation":6,"temporary_restriction":10,"admin_escalation":15,"decay_points_per_day":1}'::jsonb, false, 'Gradual punishment ladder with automatic recovery over time.'),
  ('rate_limit_defaults', '{"social_post_create":{"per_hour":3,"per_day":10},"social_comment_create":{"per_hour":20,"per_day":60},"media_upload":{"per_day":20},"avatar_upload":{"per_day":5},"social_report_create":{"per_day":10},"ai_user_chat":{"per_day":30}}'::jsonb, false, 'Default user-facing rate limits before restriction multipliers are applied.'),
  ('new_user_review_window', '{"enabled":true,"hours":24,"external_links":"review","high_risk_categories":"review"}'::jsonb, false, 'Optional cautious mode for new accounts.'),
  ('admin_ai_escalation', '{"mode":"local_then_api_if_uncertain","api_fallback_enabled":false,"admin_only":true}'::jsonb, false, 'Admin AI moderation review mode.'),
  ('user_ai_assistant_policy', '{"mode":"local_only","financial_advice":"refuse","trading_solicitation":"refuse","unsafe_content":"refuse"}'::jsonb, false, 'Policy guardrails for the learner-facing assistant.')
on conflict (key) do update set
  value = excluded.value,
  locked = excluded.locked,
  description = excluded.description,
  updated_at = now();

insert into ai_provider_settings (provider, enabled, mode, base_url, model, priority, timeout_ms, metadata) values
  ('ollama_local', false, 'user_assistant', 'http://127.0.0.1:11434', null, 10, 30000, '{"purpose":"local user assistant and local moderation guard"}'::jsonb),
  ('openai_moderation', false, 'admin_only', null, 'omni-moderation-latest', 50, 30000, '{"purpose":"optional admin-only moderation fallback"}'::jsonb),
  ('hive_moderation', false, 'admin_only', null, null, 60, 30000, '{"purpose":"optional visual moderation fallback"}'::jsonb)
on conflict (provider) do update set
  enabled = excluded.enabled,
  mode = excluded.mode,
  base_url = excluded.base_url,
  model = excluded.model,
  priority = excluded.priority,
  timeout_ms = excluded.timeout_ms,
  metadata = excluded.metadata,
  updated_at = now();

insert into ai_assistant_settings (key, enabled, value, locked, description) values
  ('user_assistant', false, '{"provider":"ollama_local","mode":"local_only","safe_learning_scope":true}'::jsonb, false, 'Learner-facing local assistant settings.'),
  ('admin_moderation_assistant', false, '{"mode":"local_then_api_if_uncertain","auto_action":false,"requires_admin_confirmation":true}'::jsonb, false, 'Admin co-moderation assistant settings.'),
  ('assistant_safety_baseline', true, '{"no_financial_advice":true,"no_trading_solicitation":true,"no_private_key_requests":true,"no_unsafe_instructions":true}'::jsonb, true, 'Non-disableable assistant safety baseline.')
on conflict (key) do update set
  enabled = excluded.enabled,
  value = excluded.value,
  locked = excluded.locked,
  description = excluded.description,
  updated_at = now();

create index if not exists social_posts_moderation_queue_idx on social_posts(moderation_status, moderation_checked_at asc, created_at asc) where moderation_status in ('pending_review', 'blocked');
create index if not exists social_comments_moderation_queue_idx on social_comments(moderation_status, moderation_checked_at asc, created_at asc) where moderation_status in ('pending_review', 'blocked');
create index if not exists media_assets_moderation_queue_idx on media_assets(moderation_status, moderation_checked_at asc, created_at asc) where moderation_status in ('pending_review', 'blocked');
create index if not exists social_posts_moderation_categories_idx on social_posts using gin (moderation_categories);
create index if not exists social_comments_moderation_categories_idx on social_comments using gin (moderation_categories);
create index if not exists media_assets_moderation_categories_idx on media_assets using gin (moderation_categories);
