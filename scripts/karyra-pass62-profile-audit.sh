#!/usr/bin/env bash
set -euo pipefail

missing=0
required_files=(
  "src/profile/mod.rs"
  "migrations/0062_profile_account_runtime.sql"
)

for file in "${required_files[@]}"; do
  if [[ ! -f "$file" ]]; then
    echo "Missing required backend file: $file" >&2
    missing=1
  fi
done

if ! grep -q 'mod profile;' src/main.rs; then
  echo "src/main.rs does not register profile module" >&2
  missing=1
fi

if ! grep -q 'nest("/v1/profile"' src/http/mod.rs; then
  echo "src/http/mod.rs does not mount /v1/profile" >&2
  missing=1
fi

if ! grep -q 'route("/me", get(me).post(update_me))' src/profile/mod.rs; then
  echo "profile /me route is not registered" >&2
  missing=1
fi

if [[ "$missing" -ne 0 ]]; then
  exit 1
fi

echo "Pass 62 backend profile audit OK"
