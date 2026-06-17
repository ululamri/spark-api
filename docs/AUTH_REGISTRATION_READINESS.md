# PASS 18A — Auth Registration Readiness Hardening

Tanggal: 2026-06-17  
Repo: `ululamri/spark-api`  
Live path: `/opt/karyra/spark-api`

## Status

PASS 18A menguatkan auth registration/login yang sudah ada agar lebih siap untuk user nyata.

Auth backend yang sudah tersedia:

```txt
GET  /v1/auth/scope
POST /v1/auth/register
POST /v1/auth/login
GET  /v1/auth/me
POST /v1/auth/logout
```

## Yang ditambahkan

Migration:

```txt
migrations/202606170011_auth_rate_limits.sql
```

Table:

```txt
auth_rate_limits
```

Rate-limit memakai bucket + SHA-256 key hash, sehingga email/client identity tidak disimpan mentah di rate-limit table.

## Guard bucket

Register:

```txt
auth_register_email   3 attempts / 1 hour
auth_register_client  20 attempts / 15 minutes, jika proxy header tersedia
```

Login:

```txt
auth_login_email      8 attempts / 15 minutes
auth_login_client     40 attempts / 15 minutes, jika proxy header tersedia
```

Client identity dibaca dari header berikut, urut:

```txt
cf-connecting-ip
x-real-ip
x-forwarded-for
```

Jika proxy header tidak ada, client bucket dilewati agar tidak mengunci semua user di satu bucket `unknown`; email bucket tetap aktif.

## Deploy backend

```bash
cd /opt/karyra/spark-api
git pull

set -a
source .env.host
set +a

psql "$DATABASE_URL" -f migrations/202606170011_auth_rate_limits.sql

cargo build --release
systemctl restart karyra-spark-api
systemctl status karyra-spark-api --no-pager
```

## Deploy frontend

Frontend patch hanya menambahkan pesan 429/rate-limit yang lebih ramah.

```bash
cd /opt/karyra/spark
git pull
pnpm build
systemctl restart karyra-spark-web
systemctl status karyra-spark-web --no-pager
```

## Smoke checks

Scope:

```bash
curl -s https://spark.user.cloudjkt01.com/v1/auth/scope | head
```

Expected phase:

```txt
auth-registration-readiness-hardening
```

Register user nyata/test:

```bash
curl -i https://spark.user.cloudjkt01.com/v1/auth/register \
  -H "content-type: application/json" \
  -d '{
    "email":"test+spark@example.com",
    "password":"change-this-password",
    "display_name":"Spark Test"
  }'
```

Expected:

```txt
HTTP 201
Set-Cookie: spark_session=...
```

Me endpoint with cookie:

```bash
curl -i https://spark.user.cloudjkt01.com/v1/auth/me \
  -H "cookie: spark_session=<cookie-from-register>"
```

DB check:

```bash
psql "$DATABASE_URL" -c "select bucket, attempt_count, window_expires_at, last_seen_at from auth_rate_limits order by last_seen_at desc limit 10;"
```

## Notes

- Email verification belum ditambahkan di PASS 18A.
- Password reset belum ditambahkan di PASS 18A.
- Auth UI `/login` dan `/register` sudah memakai backend `/v1/auth/*` dan cookie `credentials: include`.
