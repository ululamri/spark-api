#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-.env.release}"
BACKUP_DIR="${BACKUP_DIR:-backups/postgres}"

if [[ ! -f "$ENV_FILE" ]]; then
  echo "Missing env file: $ENV_FILE" >&2
  exit 1
fi

set -a
# shellcheck source=/dev/null
source "$ENV_FILE"
set +a

: "${DATABASE_URL:?DATABASE_URL is required}"

mkdir -p "$BACKUP_DIR"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="$BACKUP_DIR/spark-postgres-$STAMP.dump"

if command -v pg_dump >/dev/null 2>&1; then
  pg_dump "$DATABASE_URL" --format=custom --no-owner --no-privileges --file "$OUT"
elif command -v docker >/dev/null 2>&1; then
  docker run --rm --network host -v "$(pwd)/$BACKUP_DIR:/backup" postgres:16-alpine \
    pg_dump "$DATABASE_URL" --format=custom --no-owner --no-privileges --file "/backup/$(basename "$OUT")"
else
  echo "pg_dump or Docker is required for PostgreSQL backup" >&2
  exit 1
fi

sha256sum "$OUT" > "$OUT.sha256"
echo "PostgreSQL backup created: $OUT"
echo "Checksum: $OUT.sha256"
