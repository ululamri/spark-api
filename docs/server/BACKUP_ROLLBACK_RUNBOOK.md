# Backup and Rollback Runbook

This runbook is for early server rehearsal. It keeps Spark API recoverable before public usage.

## Backup before every risky server change

```bash
bash scripts/karyra-server-backup-postgres.sh .env.release
```

The script writes a custom-format PostgreSQL dump under `backups/postgres/` and creates a SHA-256 checksum file.

## Restore PostgreSQL backup

Restoring is destructive. Use only when you intentionally want to roll back database state.

```bash
bash scripts/karyra-server-restore-postgres.sh .env.release backups/postgres/<backup>.dump --yes-i-understand
```

## Rollback app container

For early development, rollback means returning the repo to a known good commit and rebuilding the API image:

```bash
git log --oneline -10
git checkout <known-good-commit>
docker compose --env-file .env.release -f infra/docker-compose.server.example.yml build api
docker compose --env-file .env.release -f infra/docker-compose.server.example.yml up -d api
```

Then run:

```bash
bash scripts/karyra-server-api-smoke.sh http://127.0.0.1:8787
bash scripts/karyra-server-release-summary.sh http://127.0.0.1:8787
```

## MinIO readiness

```bash
bash scripts/karyra-server-minio-readiness.sh .env.release
```

This creates the public/private buckets if missing and lists bucket state through the MinIO client container.

## Rules

- Never restore a database backup without confirming the target server.
- Never commit `.env.release`.
- Keep at least one known-good commit SHA noted before each server test.
- Do not use placeholder passwords outside examples.
- Do not expose MinIO console publicly without additional protection.
