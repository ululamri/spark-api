#!/usr/bin/env bash
set -euo pipefail

required=(
  "Cargo.toml"
  "src/main.rs"
  "src/config.rs"
  "src/state.rs"
  "src/error.rs"
  "src/http/mod.rs"
  "src/health/mod.rs"
  "src/auth/mod.rs"
  "migrations/000001_backend_foundation.sql"
  "migrations/000002_auth_backend_foundation.sql"
  "infra/docker-compose.local.yml"
  ".env.example"
)

for path in "${required[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "Missing required backend file: $path" >&2
    exit 1
  fi
done

if ! grep -q "argon2" Cargo.toml; then
  echo "Cargo.toml must include argon2 for password hashing" >&2
  exit 1
fi

if ! grep -q "SPARK_API_HOST" .env.example; then
  echo ".env.example must use SPARK_* API env keys" >&2
  exit 1
fi

if grep -R "CorsLayer::permissive" -n src 2>/dev/null; then
  echo "CORS must not stay permissive after cookie auth foundation" >&2
  exit 1
fi

if grep -R "replace(\['/', '\\\\'\]" -n src 2>/dev/null; then
  echo "Found unsafe Rust backslash replace pattern" >&2
  exit 1
fi

echo "Pass 46 auth backend audit OK"
