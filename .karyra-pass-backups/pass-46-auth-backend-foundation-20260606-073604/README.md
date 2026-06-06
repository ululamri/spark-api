# Spark API

Rust/Axum backend foundation for Karyra Spark.

## Stack

- Rust + Axum for the API service
- PostgreSQL as the primary database
- SQLx as the Rust database layer
- MinIO or Garage as self-hosted S3-compatible object storage
- Cloudflare R2/S3 as optional future migration or backup target

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

This pass is a clean backend foundation. It does not yet implement production auth, real media upload signing, or Starknet integration.
