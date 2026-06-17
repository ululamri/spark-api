# PASS 17H — Moderation Operations History API

Tanggal: 2026-06-17  
Repo: `ululamri/spark-api`  
Live path: `/opt/karyra/spark-api`

## Status

PASS 17H menambahkan API read-only untuk observability moderation operations. Modul ini tidak menjalankan action baru; hanya membaca persisted jobs/items dari PASS 17E.

Route:

```txt
GET /api/admin/social/ops/scope
GET /api/admin/social/ops/bulk-jobs
GET /api/admin/social/ops/bulk-jobs/:job_id
```

## RBAC

Semua route membutuhkan capability:

```txt
moderation_read
```

Superadmin tetap legacy/env root. Admin/moderator delegated role dapat membaca selama capability backend mengizinkan.

## Data source

```txt
social_moderation_bulk_jobs
social_moderation_bulk_job_items
```

Tidak ada table baru.

## Query filters

`GET /api/admin/social/ops/bulk-jobs` menerima:

```txt
limit       default 25, max 100
offset      default 0
status      running | dry_run | completed | partial_failed | failed
target_type post | comment | report
```

## Deploy backend

```bash
cd /opt/karyra/spark-api
git pull

set -a
source .env.host
set +a

cargo build --release
systemctl restart karyra-spark-api
systemctl status karyra-spark-api --no-pager
```

## Smoke check

Scope endpoint expected phase:

```txt
moderation-operations-history-observability
```

List endpoint expected data source:

```txt
database
```
