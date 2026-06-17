# Admin Social Moderation — PASS 17E Bulk Action Engine

Tanggal: 2026-06-17  
Scope: backend Spark API.

## Status

PASS 17E menambahkan engine bulk moderation untuk surface social admin. Engine ini berada di bawah rute:

```txt
GET  /api/admin/social/bulk/scope
POST /api/admin/social/bulk/moderation-actions
GET  /api/admin/social/bulk/jobs/:job_id
```

Model admin tetap sama:

```txt
superadmin = legacy/env root
admin      = delegated user-based role
moderator  = delegated user-based role
```

Superadmin tetap tidak dicampur ke `users` atau `admin_role_assignments`.

## Capability guard

Bulk action memakai dua lapis guard:

1. `moderation_bulk`
2. capability action spesifik:
   - `hide`, `remove` → `moderation_action`
   - `restore` → `moderation_restore`
   - `dismiss_report`, `mark_reviewed` → `reports_manage`

Ini mencegah moderator yang hanya punya `moderation_bulk` melakukan restore jika tidak punya `moderation_restore`.

## Request

```json
{
  "target_type": "post",
  "action": "hide",
  "target_ids": [
    "00000000-0000-0000-0000-000000000000"
  ],
  "reason": "spam campaign",
  "dry_run": true,
  "idempotency_key": "moderation:2026-06-17:spam-wave-1",
  "payload": {
    "source": "manual_review"
  }
}
```

Alternatif request dengan mapping report per target:

```json
{
  "target_type": "comment",
  "action": "remove",
  "targets": [
    {
      "target_id": "00000000-0000-0000-0000-000000000000",
      "report_id": "11111111-1111-1111-1111-111111111111"
    }
  ],
  "reason": "harassment",
  "dry_run": false,
  "idempotency_key": "moderation:2026-06-17:harassment-1"
}
```

`targets` diprioritaskan jika dikirim. Jika `targets` kosong/tidak ada, engine memakai `target_ids`.

## Batas dan safety

- Maksimal 100 target per request.
- Target duplikat otomatis didedup.
- `dry_run=true` memvalidasi target tanpa mengubah `social_posts`, `social_comments`, `social_reports`, atau membuat `social_moderation_actions`.
- `idempotency_key` mencegah eksekusi dobel. Request dengan key yang sama mengembalikan job lama.
- Konten dengan status `deleted` tidak diubah oleh bulk moderation.
- Target yang sudah berada di status tujuan akan diberi item status `skipped`.

## Persisted tables

Migration:

```txt
migrations/202606170008_admin_social_bulk_moderation.sql
```

Tables:

```txt
social_moderation_bulk_jobs
social_moderation_bulk_job_items
```

Job status:

```txt
running
dry_run
completed
partial_failed
failed
```

Item status:

```txt
would_apply
applied
skipped
failed
```

## Audit

Engine menulis audit:

```txt
social_bulk_moderation_job
social_bulk_moderation_item
```

Untuk eksekusi nyata (`dry_run=false`), setiap item yang berhasil juga membuat row `social_moderation_actions`, lalu menerapkan perubahan ke target.

## Deploy

```bash
cd /opt/karyra/spark-api
git pull

set -a
source .env.host
set +a

psql "$DATABASE_URL" -f migrations/202606170008_admin_social_bulk_moderation.sql

cargo build --release
systemctl restart karyra-spark-api
systemctl status karyra-spark-api --no-pager
```

## Checks

Scope:

```bash
curl -s https://spark.user.cloudjkt01.com/api/admin/social/bulk/scope \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN" | head
```

Expected phase:

```txt
advanced-social-moderation-bulk-action-engine
```

Dry-run contoh, ganti UUID dengan post nyata:

```bash
curl -s https://spark.user.cloudjkt01.com/api/admin/social/bulk/moderation-actions \
  -H "content-type: application/json" \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN" \
  -d '{
    "target_type":"post",
    "action":"hide",
    "target_ids":["00000000-0000-0000-0000-000000000000"],
    "reason":"dry run check",
    "dry_run":true,
    "idempotency_key":"manual-dry-run-001"
  }' | head
```

DB checks:

```bash
psql "$DATABASE_URL" -c "select status, dry_run, total_count, would_apply_count, applied_count, skipped_count, failed_count from social_moderation_bulk_jobs order by created_at desc limit 5;"

psql "$DATABASE_URL" -c "select target_type, action, status, count(*) from social_moderation_bulk_job_items group by target_type, action, status order by target_type, action, status;"
```
