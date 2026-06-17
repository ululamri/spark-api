# PASS 17K — Admin Audit Log API

Tanggal: 2026-06-17  
Repo: `ululamri/spark-api`  
Live path: `/opt/karyra/spark-api`

## Status

PASS 17K menambahkan API read-only untuk melihat `admin_audit_events` dari admin surface. Modul ini tidak membuat, mengubah, atau menghapus audit rows.

Route:

```txt
GET /api/admin/audit/scope
GET /api/admin/audit/events
GET /api/admin/audit/events/:event_id
```

## RBAC

Semua route membutuhkan capability:

```txt
audit_read
```

Superadmin legacy/env root tetap memiliki capability penuh. Delegated admin dapat membaca audit jika role assignment memiliki `audit_read`.

## Query filters

`GET /api/admin/audit/events` menerima:

```txt
limit       default 50, max 100
offset      default 0
actor_kind  exact match, optional
action      exact match, optional
target_type exact match, optional
```

Contoh:

```bash
curl -s "https://spark.user.cloudjkt01.com/api/admin/audit/events?limit=20&action=ml_moderation_signal_create" \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN"
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

```bash
curl -s https://spark.user.cloudjkt01.com/api/admin/audit/scope \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN" | head
```

Expected phase:

```txt
admin-audit-log-viewer
```

List events:

```bash
curl -s "https://spark.user.cloudjkt01.com/api/admin/audit/events?limit=5" \
  -H "x-karyra-admin-token: $KARYRA_ADMIN_TOKEN" | head
```
