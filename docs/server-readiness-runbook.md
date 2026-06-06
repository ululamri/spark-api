# Karyra Spark API — Server Readiness Runbook

Dokumen ini adalah runbook awal untuk memindahkan `spark-api` dari device lokal ke server yang mendukung PostgreSQL, MinIO, dan Docker.

## Status sebelum server audit

Batch backend foundation yang sudah hijau:

- Pass 45 Clean Backend
- Pass 46 Auth Backend Foundation
- Pass 47 Learning/Lab Progress API
- Pass 48 Proof Event Ledger Backend
- Pass 49 Passport Credential API
- Pass 50 Media Upload Foundation

Alur backend inti saat ini:

```txt
Auth -> Learning/Lab Progress -> Proof Event Ledger -> Passport Credential -> Media Asset Foundation
```

## Batas penting

Pass ini belum berarti production launch final.

Yang belum masuk:

- real S3/MinIO signed URL SigV4
- bucket policy hardening
- reverse proxy TLS config
- rate limiting
- email verification
- password reset
- admin/developer dashboard
- Starknet Sepolia/Mainnet integration
- NFT/non-transferable Passport mint
- public verifier

## Server setup awal

Dari server:

```bash
git clone https://github.com/ululamri/spark-api.git
cd spark-api
cp config/env.server.example .env.server
```

Edit `.env.server`:

```txt
SPARK_WEB_ORIGIN=https://domain-frontend-kamu
POSTGRES_PASSWORD=<password-kuat>
DATABASE_URL=postgres://spark:<password-kuat>@postgres:5432/spark
MINIO_ROOT_PASSWORD=<password-kuat>
SPARK_COOKIE_SECURE=true
```

## Build dan start service

```bash
bash scripts/karyra-compose-server-up.sh
```

Atau manual:

```bash
docker compose -f infra/docker-compose.server.example.yml --env-file .env.server up -d --build
```

## Migrasi database

Jika `psql` tersedia di server host:

```bash
bash scripts/karyra-db-migrate.sh
```

Alternatif dari container Postgres:

```bash
for f in migrations/*.sql; do
  docker exec -i spark-postgres psql -U spark -d spark -v ON_ERROR_STOP=1 < "$f"
done
```

Semua migration saat ini ditulis idempotent sejauh mungkin dengan `create if not exists` dan `alter table ... add column if not exists`.

## Smoke test runtime

```bash
SPARK_API_BASE_URL=http://127.0.0.1:8787 bash scripts/karyra-server-smoke-test.sh
```

Setelah database siap:

```bash
CHECK_READY=1 SPARK_API_BASE_URL=http://127.0.0.1:8787 bash scripts/karyra-server-smoke-test.sh
```

## Manual endpoint sanity check

```bash
curl http://127.0.0.1:8787/
curl http://127.0.0.1:8787/health/live
curl http://127.0.0.1:8787/health/ready
curl http://127.0.0.1:8787/v1/auth/scope
curl http://127.0.0.1:8787/v1/media/policy
```

## Auth runtime check

```bash
curl -i -X POST http://127.0.0.1:8787/v1/auth/register \
  -H 'content-type: application/json' \
  -d '{"email":"tester@example.com","password":"strongpass123","display_name":"Tester"}'
```

Pastikan response memiliki `Set-Cookie: spark_session=...; HttpOnly`.

Lalu:

```bash
curl -i http://127.0.0.1:8787/v1/auth/me \
  -H 'cookie: spark_session=<isi-cookie>'
```

## Media runtime check

Media Pass 50 masih foundation. Upload URL belum SigV4 signed URL final.

```bash
curl -i -X POST http://127.0.0.1:8787/v1/media/upload-intents \
  -H 'content-type: application/json' \
  -H 'cookie: spark_session=<isi-cookie>' \
  -d '{"purpose":"avatar","file_name":"avatar.png","mime_type":"image/png","size_bytes":12345,"visibility":"public"}'
```

## Passport runtime check

Setelah user punya proof event dari Learning/Lab:

```bash
curl -i http://127.0.0.1:8787/v1/passport/me/eligibility \
  -H 'cookie: spark_session=<isi-cookie>'
```

Passport hanya boleh terbit dari catatan sistem, bukan klaim manual user.

## CI

Pass ini menambahkan GitHub Actions workflow:

```txt
.github/workflows/backend-ci.yml
```

CI menjalankan:

```txt
cargo fmt --check
cargo check
cargo build
```

## Kapan masuk Pass berikutnya?

Setelah Pass 51 hijau, urutan aman berikutnya:

1. Pass 52 — Backend runtime hardening
   - request size limit
   - structured trace layer
   - error response consistency
   - stricter CORS/env validation

2. Pass 53 — Server DB/MinIO audit fix
   - hanya setelah dijalankan di server
   - berdasarkan error nyata dari PostgreSQL/MinIO/Docker

3. Pass 54 — Frontend API integration boundary
   - mulai hubungkan frontend ke backend auth/progress secara bertahap
   - tetap jaga UI end-user, bukan developer surface
