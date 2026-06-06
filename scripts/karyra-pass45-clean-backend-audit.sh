#!/usr/bin/env bash
set -euo pipefail

required=(
  "Cargo.toml"
  "src/main.rs"
  "src/config.rs"
  "src/http/mod.rs"
  "src/media/mod.rs"
  "src/proof/mod.rs"
  "src/passport/mod.rs"
  "migrations/000001_backend_foundation.sql"
  "infra/docker-compose.local.yml"
)

for path in "${required[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "Missing required backend file: $path" >&2
    exit 1
  fi
done

if grep -R "replace(\['/', '\\\\'\]" -n src 2>/dev/null; then
  echo "Found unsafe Rust backslash replace pattern" >&2
  exit 1
fi

echo "Pass 45 clean backend audit OK"
