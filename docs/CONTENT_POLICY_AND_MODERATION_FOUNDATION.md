# Content Policy and Moderation Foundation

**Pass:** CONTENT-POLICY-01  
**Repository:** `spark-api`  
**Scope:** public social, profile, media, safety scoring, AI moderation settings

Karyra Spark is designed as a public learning environment. The community surface supports posts, comments, profile identity, media attachments, and learner support. This moderation foundation protects that environment without turning the platform into a rigid enterprise moderation console.

The policy model separates educational blockchain discussion from exploitative or promotional behavior. Learning about Starknet, Cairo, wallet safety, and blockchain concepts is allowed. Gambling, trading solicitation, scam behavior, spam, unsafe media, and abuse are restricted.

## Goals

CONTENT-POLICY-01 adds the data foundation for:

```text
content policy categories
moderation status on posts, comments, and media
moderation events and model run logs
user safety scores
strike-based gradual punishment
restriction records
rate limit buckets
AI provider settings
learner assistant and admin assistant settings
appeals foundation
```

This pass is intentionally non-breaking. Existing public social statuses remain unchanged:

```text
published
hidden
removed
deleted
```

A separate `moderation_status` column is added to social and media records so the moderation pipeline can be rolled out gradually.

## Default policy categories

The initial taxonomy includes:

```text
sexual_minors
nsfw_explicit
gambling
trading_solicitation
financial_scam
phishing
private_key_request
malware
spam
advertising
referral_abuse
harassment
hate
violence_threat
doxxing
illegal_goods
external_link_risk
toxicity_low
```

Some categories are locked as hard safety baselines. They cannot be disabled from the future admin settings UI:

```text
sexual_minors
financial_scam
phishing
private_key_request
malware
hate
violence_threat
doxxing
```

## Allowed learning content

The policy should preserve legitimate learning activity:

```text
Starknet, Cairo, Scarb, Dojo, and smart contract learning
wallet safety education
blockchain concepts and technical discussion
questions about crypto fundamentals
security awareness and scam prevention
community learning reflections
constructive criticism
```

Educational blockchain discussion is not the same as trading solicitation. The platform should allow learning while restricting behavior that exploits new users.

## Restricted content

Restricted content includes:

```text
NSFW or explicit sexual media
gambling, slot, betting, togel, casino, or similar promotion
trading signals, pump groups, profit guarantees, and referral spam
fake airdrops, phishing, wallet drain attempts, or private key requests
spam, aggressive advertising, and unrelated promotion
harassment, hate, doxxing, and credible threats
malware, abuse tooling, and unsafe technical instructions
illegal goods or services
```

## Moderation decisions

The moderation engine uses a small decision set:

```text
allow
review
block
restrict
```

The user-facing status model uses:

```text
unreviewed
allowed
pending_review
blocked
admin_hidden
admin_removed
restored
```

For beta, existing content is backfilled as `allowed` so current public social behavior remains stable. Later passes can move new uploads and new posts through `unreviewed` or `pending_review` before they appear publicly.

## Gradual punishment model

Karyra Spark uses gradual punishment instead of immediate harsh account removal for most non-critical violations. The goal is to correct behavior while preserving access to learning.

Suggested ladder:

```text
normal
gentle_friction
reduced_velocity
probation
temporary_restriction
admin_escalation
```

The system can apply strikes based on severity. Example defaults:

```text
low-level toxicity: +1
spam or advertising: +2
trading solicitation: +3
gambling: +4
explicit NSFW media: +5
phishing, wallet secret requests, doxxing, or malware: +8 to +10
```

Strike points can decay over time. The default setting starts with:

```json
{
  "gentle_friction": 1,
  "reduced_velocity": 3,
  "probation": 6,
  "temporary_restriction": 10,
  "admin_escalation": 15,
  "decay_points_per_day": 1
}
```

## Rate limit foundation

Rate limiting is tracked per user and per feature scope. This allows the platform to reduce velocity for abusive behavior without blocking learning access.

Initial feature scopes include:

```text
social_post_create
social_comment_create
social_reaction_create
social_report_create
media_upload
avatar_upload
profile_update
follow_user
ai_user_chat
admin_ai_review
```

Default beta limits are stored in `content_policy_settings`:

```json
{
  "social_post_create": { "per_hour": 3, "per_day": 10 },
  "social_comment_create": { "per_hour": 20, "per_day": 60 },
  "media_upload": { "per_day": 20 },
  "avatar_upload": { "per_day": 5 },
  "social_report_create": { "per_day": 10 },
  "ai_user_chat": { "per_day": 30 }
}
```

Future enforcement should keep reading and lesson access available whenever possible. Restrictions should primarily affect posting, commenting, media uploads, reports, following, and assistant usage.

## AI moderation foundation

CONTENT-POLICY-01 adds settings tables only. It does not call local or external AI yet.

Provider settings are prepared for:

```text
ollama_local
openai_moderation
hive_moderation
```

Recommended beta posture:

```text
user assistant: local only
admin moderation assistant: local + optional API fallback
external API: admin-only and disabled by default
auto action: disabled
admin confirmation: required
```

AI should support moderation decisions. It should not become the final authority for destructive actions.

## Admin UI implications

The future Admin Moderation UI should include:

```text
Reports queue
Posts queue
Comments queue
Media queue
Policy settings
Rate limit settings
AI provider settings
User safety profile
Moderation events
Model run history
Appeals
```

The admin page should allow configuration of flexible thresholds, rate limits, and AI modes, while keeping hard safety baselines locked.

## Migration

The migration file is:

```text
migrations/0079_content_policy_foundation.sql
```

It adds:

```text
content_policy_categories
content_policy_settings
moderation_events
moderation_model_runs
user_safety_scores
moderation_strikes
user_restrictions
user_rate_limit_buckets
ai_provider_settings
ai_assistant_settings
moderation_appeals
```

It also adds moderation columns to:

```text
social_posts
social_comments
media_assets
```

## Next passes

Recommended order after this foundation:

```text
CONTENT-POLICY-02 — rule-based moderation and strike engine
CONTENT-POLICY-03 — adaptive rate limit enforcement
CONTENT-POLICY-04 — media moderation pipeline
AI-MODERATION-01 — local AI and admin API fallback foundation
ADMIN-MOD-UI-01 — admin gate, API client, settings shell
ADMIN-MOD-UI-02 — reports queue and moderation actions
ADMIN-MOD-UI-03 — settings, AI triage, audit logs, and responsive polish
```
