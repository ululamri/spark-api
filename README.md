# Spark API

Rust/Axum backend foundation for Karyra Spark.

## Stack

- Rust + Axum for the API service
- PostgreSQL as the primary database
- SQLx as the Rust database layer
- Argon2 for password hashing
- httpOnly cookie sessions
- MinIO or Garage as self-hosted S3-compatible object storage
- Cloudflare R2/S3 as optional future migration or backup target

## Implemented API surfaces

```txt
GET  /health/live
GET  /health/ready
POST /v1/auth/register
POST /v1/auth/login
GET  /v1/auth/me
POST /v1/auth/logout
GET  /v1/auth/scope
/v1/profile
/v1/learning
/v1/lab
/v1/media
/v1/proof
/v1/passport
/v1/community
/v1/hub
/v1/social
/api/admin/*
```

`register` and `login` set an httpOnly `spark_session` cookie. The raw session token is never stored in the database; only a SHA-256 token hash is stored in `sessions.token_hash`.

## Local run

```bash
cp .env.example .env
docker compose -f infra/docker-compose.local.yml up -d postgres minio
cargo fmt --check
cargo check
cargo run
```

Health checks:

```bash
curl http://127.0.0.1:8787/health/live
curl http://127.0.0.1:8787/health/ready
```

Without PostgreSQL, `cargo check` and `cargo build` can still pass. Runtime startup uses a lazy SQLx pool, but DB-backed endpoints and `/health/ready` need PostgreSQL.

## Live deployment

See [docs/LIVE_DEPLOYMENT.md](docs/LIVE_DEPLOYMENT.md) for the current non-Docker live deployment contract.

## Known production gaps

The current production-readiness backlog still includes email verification, password reset, account recovery, full session/device management, and stricter server-side media upload completion validation.

## License

Karyra Spark API is open source under the MIT License. See [LICENSE](LICENSE).
