-- PASS AI-MODERATION-01 — AI moderation and local assistant runtime settings
-- Secrets are not stored in the database. API keys must be supplied from host env.

insert into ai_provider_settings (provider, enabled, mode, base_url, model, priority, timeout_ms, metadata) values
  (
    'ollama_local',
    false,
    'user_assistant',
    'http://127.0.0.1:11434',
    'llama-guard3:1b',
    10,
    30000,
    '{"purpose":"local learner assistant and local moderation guard","stores_secret":false,"default_enabled":false}'::jsonb
  ),
  (
    'openai_moderation',
    false,
    'admin_only',
    null,
    'omni-moderation-latest',
    50,
    30000,
    '{"purpose":"optional admin-only external moderation fallback","stores_secret":false,"secret_env":"OPENAI_API_KEY","default_enabled":false}'::jsonb
  )
on conflict (provider) do update set
  mode = excluded.mode,
  base_url = coalesce(ai_provider_settings.base_url, excluded.base_url),
  model = coalesce(ai_provider_settings.model, excluded.model),
  priority = excluded.priority,
  timeout_ms = excluded.timeout_ms,
  metadata = ai_provider_settings.metadata || excluded.metadata,
  updated_at = now();

insert into ai_assistant_settings (key, enabled, value, locked, description) values
  (
    'user_assistant',
    false,
    '{"provider":"ollama_local","mode":"local_only","safe_learning_scope":true,"auto_action":false}'::jsonb,
    false,
    'Learner-facing local assistant. Disabled by default until a local model is installed and verified.'
  ),
  (
    'admin_moderation_assistant',
    false,
    '{"mode":"local_then_api_if_uncertain","local_provider":"ollama_local","external_provider":"openai_moderation","external_api_admin_only":true,"auto_action":false,"requires_admin_confirmation":true}'::jsonb,
    false,
    'Admin co-moderation assistant. AI can summarize and recommend, but cannot execute destructive moderation actions.'
  ),
  (
    'assistant_safety_baseline',
    true,
    '{"no_financial_advice":true,"no_trading_solicitation":true,"no_private_key_requests":true,"no_unsafe_instructions":true,"admin_confirmation_required":true}'::jsonb,
    true,
    'Non-disableable assistant safety baseline.'
  )
on conflict (key) do update set
  value = case when ai_assistant_settings.locked then ai_assistant_settings.value else excluded.value end,
  description = excluded.description,
  updated_at = now();

insert into content_policy_settings (key, value, locked, description) values
  (
    'ai_moderation_runtime',
    '{"version":"ai-moderation-01","learner_external_api":false,"admin_external_api_default":false,"auto_action":false,"local_first":true}'::jsonb,
    false,
    'Runtime AI moderation posture for local assistant and admin co-moderation.'
  ),
  (
    'ai_moderation_thresholds',
    '{"local_review_score":0.55,"local_block_score":0.85,"external_review_score":0.55,"external_block_score":0.90,"fallback_when_rule_review":true}'::jsonb,
    false,
    'Default AI moderation thresholds for admin review triage.'
  )
on conflict (key) do update set
  value = excluded.value,
  locked = excluded.locked,
  description = excluded.description,
  updated_at = now();

create index if not exists moderation_model_runs_created_idx
  on moderation_model_runs(created_at desc);

create index if not exists moderation_model_runs_decision_idx
  on moderation_model_runs(decision, score desc, created_at desc);
