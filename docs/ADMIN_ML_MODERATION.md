# Admin ML Moderation — PASS 17F Signal Pipeline

Tanggal: 2026-06-17  
Scope: backend Spark API, social moderation.

## Status

PASS 17F menambahkan pipeline ML moderation signal untuk konten social. Pipeline ini bersifat **human-in-the-loop**:

```txt
ML/rules memberi signal -> admin/moderator review -> bulk/single moderation action manual
```

Pipeline ini **tidak auto-delete**, **tidak auto-remove**, dan **tidak auto-restore** konten.

## Route

```txt
GET  /api/admin/social/ml/scope
GET  /api/admin/social/ml/signals
POST /api/admin/social/ml/scan
POST /api/admin/social/ml/scan-batch
POST /api/admin/social/ml/signals/:signal_id/mark-reviewed
```

## RBAC

Read:

```txt
moderation_read
```

Scan / mark reviewed:

```txt
ml_moderation_manage
```

Superadmin tetap legacy/env root. Admin delegated role bisa diberi `ml_moderation_manage`. Moderator default tidak diberi capability ini kecuali nanti sengaja didelegasikan.

## Signal status

```txt
clean
needs_review
high_risk
blocked_pending_review
```

Makna:

- `clean`: tidak ada keberatan safety dari rules/ML.
- `needs_review`: perlu review manusia.
- `high_risk`: prioritas review tinggi.
- `blocked_pending_review`: indikasi risiko sangat tinggi, tetap menunggu keputusan manusia.

## Sources

```txt
rules
local_ai
external_ai
combined
```

Default scan:

- Rules selalu jalan.
- Local AI dicoba jika provider `ollama_local` aktif.
- External fallback default `false`; hanya jalan jika request mengaktifkan dan provider aktif.
- Jika provider AI error/offline, scan tetap menghasilkan rule-based signal dan menulis provider warning di metadata.

## Scan satu target

```bash
curl -s https://spark.user.cloudjkt01.com/api/admin/social/ml/scan \
  -H "content-type: application/json" \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN" \
  -d '{
    "target_type":"post",
    "target_id":"00000000-0000-0000-0000-000000000000",
    "use_local_ai":false,
    "use_external_fallback":false,
    "note":"pass 17f smoke test"
  }'
```

## Scan batch

```bash
curl -s https://spark.user.cloudjkt01.com/api/admin/social/ml/scan-batch \
  -H "content-type: application/json" \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN" \
  -d '{
    "target_type":"post",
    "status":"published",
    "limit":5,
    "use_local_ai":false,
    "use_external_fallback":false,
    "note":"batch smoke test"
  }'
```

## List signals

```bash
curl -s "https://spark.user.cloudjkt01.com/api/admin/social/ml/signals?limit=10" \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN"
```

Filter:

```txt
?target_type=post
?target_type=comment
?status=clean
?status=needs_review
?status=high_risk
?status=blocked_pending_review
```

## Mark reviewed

```bash
curl -s https://spark.user.cloudjkt01.com/api/admin/social/ml/signals/$SIGNAL_ID/mark-reviewed \
  -H "content-type: application/json" \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN" \
  -d '{"note":"reviewed manually"}'
```

## Tables

Migration:

```txt
migrations/202606170010_ml_moderation_signal_pipeline.sql
```

Table:

```txt
social_moderation_ml_signals
```

Related evidence:

```txt
moderation_events
moderation_model_runs
admin_audit_events
```

## Deploy

```bash
cd /opt/karyra/spark-api
git pull

set -a
source .env.host
set +a

psql "$DATABASE_URL" -f migrations/202606170010_ml_moderation_signal_pipeline.sql

cargo build --release
systemctl restart karyra-spark-api
systemctl status karyra-spark-api --no-pager
```

## Smoke check

Scope:

```bash
curl -s https://spark.user.cloudjkt01.com/api/admin/social/ml/scope \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN" | head
```

Expected phase:

```txt
ml-moderation-signal-pipeline
```

Pick one post and scan without external/local AI to test DB path only:

```bash
POST_ID=$(psql -At "$DATABASE_URL" -c "select id from social_posts where status = 'published' limit 1;")

curl -s https://spark.user.cloudjkt01.com/api/admin/social/ml/scan \
  -H "content-type: application/json" \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN" \
  -d "{
    \"target_type\":\"post\",
    \"target_id\":\"$POST_ID\",
    \"use_local_ai\":false,
    \"use_external_fallback\":false,
    \"note\":\"pass 17f smoke test\"
  }" | head
```

DB checks:

```bash
psql "$DATABASE_URL" -c "select target_type, status, decision, severity, count(*) from social_moderation_ml_signals group by target_type, status, decision, severity order by target_type, status, decision, severity;"

psql "$DATABASE_URL" -c "select actor_kind, action, target_type, target_id, created_at from admin_audit_events where action like 'ml_moderation%' order by created_at desc limit 10;"
```
