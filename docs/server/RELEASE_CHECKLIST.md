# Spark API Release Checklist

Use this checklist before the first full server audit.

## Repository

- [ ] Latest backend pass is pushed to GitHub.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo check` passes.
- [ ] `cargo build` passes.
- [ ] CI, if enabled, is green.

## Server env

- [ ] `.env.release` exists only on the server.
- [ ] No `replace_with_*`, `example.com`, or `change_me` values remain.
- [ ] `SPARK_WEB_ORIGIN` points to the intended frontend origin.
- [ ] `SPARK_COOKIE_SECURE=true` for HTTPS production.
- [ ] PostgreSQL password is strong.
- [ ] MinIO root password is strong.

## Database

- [ ] PostgreSQL container is healthy.
- [ ] All migrations are applied in order.
- [ ] PostgreSQL backup succeeds before API traffic.

## Storage

- [ ] MinIO container starts.
- [ ] Public/private buckets exist.
- [ ] Buckets are not accidentally public except intended public assets.

## API smoke

- [ ] `/health/live` returns OK.
- [ ] `/health/ready` checks database readiness.
- [ ] Auth register/login/me/logout smoke passes.
- [ ] Learning/Lab/Proof/Passport smoke passes.
- [ ] Media policy and upload intent smoke passes.

## Rollback

- [ ] Known-good commit SHA is written down.
- [ ] PostgreSQL backup file exists.
- [ ] Restore procedure is understood but not run unless needed.
