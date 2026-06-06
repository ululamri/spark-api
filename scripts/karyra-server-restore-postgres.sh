#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-}"
BACKUP_FILE="${2:-}"
CONFIRM="${3:-}"

if [[ -z "$ENV_FILE" || -z "$BACKUP_FILE" ]]; then
  echo "Usage: $0 .env.release backups/postgres/<backup>.dump --yes-i-understand" >&2
  exit 1
fi

if [[ "$CONFIRM" != "--yes-i-understand" ]]; then
  echo "Restore is destructive. Re-run with --yes-i-understand as the third argument." >&2
  exit 1
fi

if [[ ! -f "$ENV_FILE" ]]; then
  echo "Missing env file: $ENV_FILE" >&2
  exit 1
fi

if [[ ! -f "$BACKUP_FILE" ]]; then
  echo "Missing backup file: $BACKUP_FILE" >&2
  exit 1
fi

set -a
# shellcheck source=/dev/null
source "$ENV_FILE"
set +a

: "${DATABASE_URL:?DATABASE_URL is required}"

if [[ -f "$BACKUP_FILE.sha256" ]]; then
  sha256sum -c "$BACKUP_FILE.sha256"
fi

if command -v pg_restore >/dev/null 2>&1; then
  pg_restore --clean --if-exists --no-owner --no-privileges --dbname "$DATABASE_URL" "$BACKUP_FILE"
elif command -v docker >/dev/null 2>&1; then
  docker run --rm --network host -v "$(pwd)/$(dirname "$BACKUP_FILE"):/backup" postgres:16-alpine \
    pg_restore --clean --if-exists --no-owner --no-privileges --dbname "$DATABASE_URL" "/backup/$(basename "$BACKUP_FILE")"
else
  echo "pg_restore or Docker is required for PostgreSQL restore" >&2
  exit 1
fi

echo "PostgreSQL restore completed from: $BACKUP_FILE"
