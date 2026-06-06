#!/usr/bin/env bash
set -euo pipefail

DATABASE_URL_INPUT="${1:-${DATABASE_URL:-}}"

if [[ -z "$DATABASE_URL_INPUT" ]]; then
  echo "DATABASE_URL is required. Usage: bash scripts/karyra-server-migration-check.sh \"$DATABASE_URL\"" >&2
  exit 1
fi

if ! command -v psql >/dev/null 2>&1; then
  echo "psql is required for migration check" >&2
  exit 1
fi

if [[ ! -d migrations ]]; then
  echo "migrations directory not found" >&2
  exit 1
fi

for migration in migrations/*.sql; do
  echo "Applying migration: $migration"
  psql "$DATABASE_URL_INPUT" -v ON_ERROR_STOP=1 -f "$migration" >/dev/null
done

required_tables=(
  users
  profiles
  sessions
  lesson_progress
  checkpoint_results
  lab_attempts
  proof_events
  passport_credentials
  media_assets
  media_links
)

for table in "${required_tables[@]}"; do
  exists="$(psql "$DATABASE_URL_INPUT" -tAc "select to_regclass('public.${table}') is not null")"
  if [[ "$exists" != "t" ]]; then
    echo "Missing required table after migrations: $table" >&2
    exit 1
  fi
done

required_columns=(
  "sessions.revoked_at"
  "sessions.last_seen_at"
  "proof_events.event_hash"
  "proof_events.evidence_root"
  "passport_credentials.credential_hash"
  "passport_credentials.evidence_event_count"
  "media_assets.status"
  "media_links.entity_type"
)

for item in "${required_columns[@]}"; do
  table="${item%%.*}"
  column="${item##*.}"
  exists="$(psql "$DATABASE_URL_INPUT" -tAc "select exists (select 1 from information_schema.columns where table_schema = 'public' and table_name = '${table}' and column_name = '${column}')")"
  if [[ "$exists" != "t" ]]; then
    echo "Missing required column after migrations: $item" >&2
    exit 1
  fi
done

echo "Server migration check OK"
