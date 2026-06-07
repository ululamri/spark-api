# Pass 68I — Unified Deployment Stack for Spark + API + Hub

Pass ini menyiapkan stack beta satu domain untuk tiga repo:

- `spark` sebagai aplikasi readiness gateway publik.
- `spark-api` sebagai backend Rust/Axum.
- `hub` sebagai guided Starknet ecosystem gateway.

## Topology beta satu domain

```text
https://spark.user.cloudjkt01.com/       -> spark-web
https://spark.user.cloudjkt01.com/hub/   -> spark-hub
https://spark.user.cloudjkt01.com/v1/*   -> spark-api
https://spark.user.cloudjkt01.com/health -> spark-api
```

## Kenapa Hub di `/hub`

Domain beta belum mendukung subdomain terpisah. Karena itu Hub tetap repo terpisah, tetapi disajikan di bawah path `/hub` pada domain Spark. Saat rilis produksi penuh, topology ini bisa dipindah menjadi domain/subdomain terpisah tanpa menggabungkan repo.

## Prinsip tooling

Frontend Spark dan Hub memakai `pnpm`. Dockerfile staging untuk Spark dan Hub memakai Corepack + pnpm, bukan npm.

## Deploy manual CLI

Dari server, struktur direktori yang diharapkan:

```text
/opt/karyra/spark
/opt/karyra/spark-api
/opt/karyra/hub
```

Jalankan dari repo backend:

```bash
cd /opt/karyra/spark-api
docker compose --env-file config/env.unified.staging -f infra/docker-compose.unified.staging.yml ps
docker compose --env-file config/env.unified.staging -f infra/docker-compose.unified.staging.yml up -d --build spark-api
docker compose --env-file config/env.unified.staging -f infra/docker-compose.unified.staging.yml up -d --build spark-web
docker compose --env-file config/env.unified.staging -f infra/docker-compose.unified.staging.yml up -d --build spark-hub
docker compose --env-file config/env.unified.staging -f infra/docker-compose.unified.staging.yml up -d caddy
```

## Smoke manual

```bash
curl -I https://spark.user.cloudjkt01.com/
curl -I https://spark.user.cloudjkt01.com/hub/
curl -I https://spark.user.cloudjkt01.com/health
```

Yang diharapkan:

- Spark terbuka di `/`.
- Hub terbuka di `/hub/`.
- API health tetap merespons.
- Link dari Spark ke Hub tetap berada di satu domain.
