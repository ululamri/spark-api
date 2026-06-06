# Spark API Server Runtime Checklist

Use this checklist when moving from local-only build checks to a real server.

## 1. Environment

- [ ] `APP_ENV=production`
- [ ] `SPARK_API_HOST=0.0.0.0`
- [ ] `SPARK_WEB_ORIGIN` points to the real frontend origin
- [ ] `SPARK_COOKIE_SECURE=true` behind HTTPS
- [ ] `POSTGRES_PASSWORD` is replaced with a strong secret
- [ ] `MINIO_ROOT_PASSWORD` is replaced with a strong secret
- [ ] No `example.com`, `replace_with`, or `change_me` placeholder remains
- [ ] `.env.server` is not committed to Git

Run:

```bash
bash scripts/karyra-server-env-audit.sh .env.server
```

## 2. Runtime containers

- [ ] PostgreSQL starts and passes healthcheck
- [ ] MinIO/Garage starts
- [ ] Spark API container starts
- [ ] Spark API container runs as non-root user
- [ ] Healthcheck is enabled

Run:

```bash
docker compose --env-file .env.server -f infra/docker-compose.server.example.yml ps
```

## 3. Migrations

- [ ] All migrations apply successfully
- [ ] Required tables exist
- [ ] Required columns exist

Run:

```bash
source .env.server
bash scripts/karyra-server-migration-check.sh "$DATABASE_URL"
```

## 4. API smoke

- [ ] `/health/live` returns 200
- [ ] `/health/ready` returns 200
- [ ] `/v1/auth/scope` returns 200
- [ ] `/v1/learning/scope` returns 200
- [ ] `/v1/lab/scope` returns 200
- [ ] `/v1/proof/scope` returns 200
- [ ] `/v1/passport/scope` returns 200
- [ ] `/v1/media/policy` returns 200

Run:

```bash
bash scripts/karyra-server-api-smoke.sh http://127.0.0.1:8787
```

## 5. Auth → Proof → Passport smoke

- [ ] Register disposable user
- [ ] Session cookie works
- [ ] Learning record is accepted
- [ ] Lab record is accepted
- [ ] Proof evidence root is produced
- [ ] Passport eligibility returns eligible
- [ ] Passport credential can be issued

Run:

```bash
bash scripts/karyra-server-auth-proof-passport-smoke.sh http://127.0.0.1:8787
```

## 6. Hold before public deploy

Before opening to public users, confirm:

- [ ] HTTPS/proxy configured
- [ ] CORS origin set to production frontend only
- [ ] Secure cookie enabled
- [ ] Logs do not expose passwords/tokens
- [ ] Backups exist for PostgreSQL data
- [ ] Object storage volume is persistent
- [ ] Server firewall allows only intended ports
- [ ] Admin/developer surfaces remain hidden from public UI
