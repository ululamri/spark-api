#!/usr/bin/env bash
set -euo pipefail

required=(
  "Cargo.toml"
  "Cargo.lock"
  "src/main.rs"
  "src/config.rs"
  "src/state.rs"
  "src/auth/mod.rs"
  "src/auth/session.rs"
  "src/learning/mod.rs"
  "src/lab/mod.rs"
  "src/proof/mod.rs"
  "src/proof/ledger.rs"
  "src/passport/mod.rs"
  "src/media/mod.rs"
  "migrations/000001_backend_foundation.sql"
  "migrations/000002_auth_backend_foundation.sql"
  "migrations/000003_learning_lab_progress.sql"
  "migrations/000004_proof_event_ledger.sql"
  "migrations/000005_passport_credential_api.sql"
  "migrations/000006_media_upload_foundation.sql"
  "Dockerfile"
  "infra/docker-compose.server.example.yml"
  "config/env.server.example"
  "scripts/karyra-db-migrate.sh"
  "scripts/karyra-server-smoke-test.sh"
  "docs/server-readiness-runbook.md"
  ".github/workflows/backend-ci.yml"
)

for path in "${required[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "Missing required Pass 51 file: $path" >&2
    exit 1
  fi
done

for key in \
  "SPARK_API_HOST" \
  "SPARK_API_PORT" \
  "SPARK_WEB_ORIGIN" \
  "DATABASE_URL" \
  "S3_ENDPOINT" \
  "S3_BUCKET_PUBLIC" \
  "S3_BUCKET_PRIVATE" \
  "SPARK_SESSION_COOKIE" \
  "SPARK_COOKIE_SECURE"; do
  if ! grep -q "^${key}=" config/env.server.example; then
    echo "Missing server env key: $key" >&2
    exit 1
  fi
done

if grep -R "replace_with_strong_password" .env .env.server 2>/dev/null; then
  echo "Unsafe placeholder secret found in active env file. Replace secrets before server runtime." >&2
  exit 1
fi

if command -v cargo >/dev/null 2>&1; then
  cargo fmt --check
  cargo check
  cargo build
else
  echo "cargo not found; skipping Rust checks"
fi

echo "Pass 51 server readiness audit OK"
