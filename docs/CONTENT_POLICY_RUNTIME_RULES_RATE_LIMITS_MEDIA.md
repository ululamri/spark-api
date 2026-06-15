# Content Policy Runtime: Rules, Rate Limits, and Media Quarantine

**Pass:** CONTENT-POLICY-02 + CONTENT-POLICY-03 + CONTENT-POLICY-04  
**Repository:** `spark-api`  
**Scope:** public social writes, adaptive limits, strikes, restrictions, media upload quarantine

This pass activates the first runtime layer of the Karyra Spark content policy system. It builds on the CONTENT-POLICY-01 foundation and keeps the rollout intentionally conservative: rules are deterministic, external AI is not called yet, and destructive actions remain reviewable by administrators.

## What is active now

```text
rule-based text moderation for social posts and comments
adaptive rate limits for social actions
strike logging and safety score updates
restriction records for repeated violations
feed filtering by moderation status
media upload rate limits
media metadata quarantine before public use
safe error responses for policy/rate-limit failures
```

## Social moderation behavior

New posts and comments are scanned before publication.

The runtime decision model is:

```text
allow  -> publish normally
review -> save as hidden and pending_review
block  -> reject the write and record a moderation event
```

Public feed queries only return content with:

```text
status = published
moderation_status = allowed or restored
```

This means content held for review or blocked by policy does not appear in the public community feed.

## Restricted content categories

The first rule set covers:

```text
unsafe media indicators
gambling and betting promotion
trading solicitation and pump/referral behavior
wallet-drain and fake claim patterns
private key or seed phrase requests
malware or abuse tooling outside an educational safety context
credible threats
doxxing indicators
advertising/referral abuse
risky external links
low-level toxicity
```

The rule set is intentionally conservative. Borderline educational blockchain content should not be blocked solely because it discusses wallets, security, Starknet, Cairo, or crypto concepts.

## Adaptive rate limits

Rate limits are enforced per user and per feature scope. Current scopes include:

```text
social_post_create
social_comment_create
social_reaction_create
social_report_create
follow_user
media_upload
avatar_upload
```

Default social limits:

```text
posts: 3/hour and 10/day
comments: 20/hour and 60/day
reactions: 100/hour and 500/day
reports: 10/day
follows: 30/day
```

Default media limits:

```text
media uploads: 20/day
avatar uploads: 5/day
```

Restriction levels reduce effective limits gradually:

```text
gentle_friction -> 75% of normal velocity
reduced_velocity -> 50% of normal velocity
probation -> 25% of normal velocity
temporary_restriction / hard_block -> write action denied
```

Learning access should remain open whenever possible. Restrictions target write-heavy or abuse-prone actions, not reading or lesson progress.

## Strike and punishment model

Policy matches can create moderation events and strikes. Strike points update:

```text
user_safety_scores
moderation_strikes
user_restrictions
```

The ladder remains gradual:

```text
normal
gentle_friction
reduced_velocity
probation
temporary_restriction
admin_escalation
```

The system is designed to allow recovery. Strike decay and admin override controls are expected in later admin UI passes.

## Media quarantine behavior

Media upload intents are rate-limited at the database layer. When an asset is marked as uploaded, media metadata is scanned for high-risk indicators.

If the asset is allowed:

```text
moderation_status = allowed
visibility remains public/private according to purpose
```

If the asset is blocked or requires review:

```text
moderation_status = blocked or pending_review
visibility is changed to private
public_url is cleared
```

This prevents risky uploaded assets from being used as avatars or social attachments, because profile/social attachment code requires an uploaded public asset that remains policy-allowed.

## Important limitation

This pass does not perform pixel-level image classification. The current media layer can quarantine based on metadata and upload context. Full image safety requires the next AI/local-scanner layer.

Recommended next implementation:

```text
AI-MODERATION-01 — local AI and admin API fallback foundation
server-side image scanner adapter
admin moderation queues for pending media/posts/comments
admin settings for thresholds and providers
```

## Error behavior

The API now maps policy/rate-limit database exceptions into user-safe responses instead of returning generic internal errors.

Rate-limited actions return:

```text
HTTP 429 Too Many Requests
```

Policy failures return:

```text
HTTP 400 Bad Request
```

## Files changed

```text
src/moderation/mod.rs
src/social/mod.rs
src/error.rs
src/main.rs
migrations/0080_content_policy_runtime_media.sql
```

## Deployment order

Run migrations before restarting the API:

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

## Suggested smoke checks

```bash
curl -s https://spark.user.cloudjkt01.com/v1/social/scope | jq
curl -s https://spark.user.cloudjkt01.com/v1/media/policy | jq
```

After logging in through a browser session, test:

```text
normal post -> appears in feed
high-risk promotion -> blocked or hidden for review
excessive post/comment attempts -> 429 response
risky media metadata -> asset becomes private/pending or blocked
```
