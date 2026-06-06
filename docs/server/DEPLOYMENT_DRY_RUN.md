# Server Deployment Dry-Run

Pass 53 prepares Spark API for a controlled server rehearsal. It is not a production launch checklist yet. The goal is to prove that the repo can be built, configured, started, migrated, smoked, backed up, and rolled back without relying on the current local device.

## 1. Prepare server workspace

```bash
git clone https://github.com/ululamri/spark-api.git
cd spark-api
cp config/release.env.example .env.release
```

Edit `.env.release` and replace every placeholder value. Do not commit `.env.release`.

## 2. Dry-run without starting production traffic

```bash
bash scripts/karyra-server-deploy-dry-run.sh .env.release
```

The dry-run checks:

- required environment variables
- placeholder secrets
- Docker and Docker Compose availability
- rendered compose config
- API image build
- migration file presence
- optional live `/health/live` response if API is already running

## 3. Start stack when dry-run is clean

```bash
docker compose --env-file .env.release -f infra/docker-compose.server.example.yml up -d postgres minio
# Run migrations with the existing Pass 51/52 migration helper before API traffic.
docker compose --env-file .env.release -f infra/docker-compose.server.example.yml up -d api
```

## 4. Smoke test

```bash
bash scripts/karyra-server-api-smoke.sh http://127.0.0.1:8787
bash scripts/karyra-server-auth-proof-passport-smoke.sh http://127.0.0.1:8787
bash scripts/karyra-server-release-summary.sh http://127.0.0.1:8787
```

## 5. Stop rehearsal stack

```bash
docker compose --env-file .env.release -f infra/docker-compose.server.example.yml down
```

Do not remove volumes unless you intentionally want to delete PostgreSQL and MinIO data.
