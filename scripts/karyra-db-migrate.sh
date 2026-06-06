#!/usr/bin/env bash
set -euo pipefail

if [[ -f .env.server ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env.server
  set +a
elif [[ -f config/env.server.example ]]; then
  echo "Using config/env.server.example. For real server use, copy it to .env.server and replace secrets." >&2
  set -a
  # shellcheck disable=SC1091
  source config/env.server.example
  set +a
fi

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "DATABASE_URL is required" >&2
  exit 1
fi

if ! command -v psql >/dev/null 2>&1; then
  echo "psql is required to run migrations from this script" >&2
  exit 1
fi

for migration in migrations/*.sql; do
  echo "Applying $migration"
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$migration"
done

echo "Database migrations applied"
