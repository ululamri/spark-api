# AI Moderation Runtime Foundation

**Pass:** AI-MODERATION-01  
**Repository:** `spark-api`

This pass adds the first AI runtime foundation for Karyra Spark. AI is an assistant layer only. It can help learners, support admin review, and produce recommendations, but it cannot execute destructive moderation actions.

## Runtime posture

```text
Learner assistant: local only
Admin AI: local first, optional external fallback
External API: admin-only and disabled by default
Auto action: disabled
Admin confirmation: required
Secrets: host environment only, not stored in database
```

## New routes

```text
GET  /v1/ai/scope
POST /v1/ai/user-assistant/chat
GET  /api/admin/ai/scope
GET  /api/admin/ai/settings
POST /api/admin/ai/settings
POST /api/admin/ai/moderate-text
```

## Learner assistant

The learner assistant is for lesson support, Cairo and Starknet explanation, wallet safety education, platform navigation, and beginner-friendly learning guidance.

Safeguards:

```text
authenticated learner session
adaptive ai_user_chat rate limit
prompt screening before local model call
local provider only
no external API call for learner chat
```

## Admin AI co-moderation

Admin AI text review creates an `ai_review` moderation event and can run:

```text
rule-based policy signal
local AI classifier
optional external moderation fallback
```

The result returns the rule decision, AI results, final recommendation, categories, and score. Recommendations are informational and still require explicit admin confirmation.

## Provider settings

Settings are stored in:

```text
ai_provider_settings
ai_assistant_settings
content_policy_settings
```

Initial providers:

```text
ollama_local
openai_moderation
hive_moderation
```

API keys are not stored through the admin settings endpoint.

## Migration

```text
migrations/0081_ai_moderation_runtime.sql
```

## Files changed

```text
Cargo.toml
src/config.rs
src/ai_runtime.rs
src/ai.rs
src/admin_ai.rs
src/main.rs
src/http/mod.rs
migrations/0081_ai_moderation_runtime.sql
docs/AI_MODERATION_RUNTIME_FOUNDATION.md
```

## Deployment order

```bash
cd /opt/karyra/spark-api
git pull
set -a
source .env.host
set +a
sqlx migrate info
sqlx migrate run
cargo build --release
systemctl restart karyra-spark-api
systemctl status karyra-spark-api --no-pager
```

## Next pass

```text
ADMIN-MOD-UI-01 — admin gate, API client, settings shell
ADMIN-MOD-UI-02 — reports queue and moderation actions
ADMIN-MOD-UI-03 — settings, AI triage, audit logs, responsive polish
```
