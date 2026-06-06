#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-.env.release}"
COMPOSE_FILE="${COMPOSE_FILE:-infra/docker-compose.server.example.yml}"
API_BASE_URL="${API_BASE_URL:-http://127.0.0.1:8787}"

if [[ ! -f "$ENV_FILE" ]]; then
  echo "Missing env file: $ENV_FILE" >&2
  echo "Create one from config/release.env.example and replace all placeholder values." >&2
  exit 1
fi

if [[ ! -f "$COMPOSE_FILE" ]]; then
  echo "Missing compose file: $COMPOSE_FILE" >&2
  exit 1
fi

if grep -E "replace_with_|example\.com|change_me" "$ENV_FILE" >/dev/null; then
  echo "Env file still contains placeholder values. Edit $ENV_FILE first." >&2
  exit 1
fi

set -a
# shellcheck source=/dev/null
source "$ENV_FILE"
set +a

required_vars=(
  APP_ENV
  SPARK_API_HOST
  SPARK_API_PORT
  SPARK_WEB_ORIGIN
  DATABASE_URL
  POSTGRES_DB
  POSTGRES_USER
  POSTGRES_PASSWORD
  S3_ENDPOINT
  S3_BUCKET_PUBLIC
  S3_BUCKET_PRIVATE
  MINIO_ROOT_USER
  MINIO_ROOT_PASSWORD
  SPARK_SESSION_COOKIE
  SPARK_COOKIE_SECURE
)

for key in "${required_vars[@]}"; do
  if [[ -z "${!key:-}" ]]; then
    echo "Missing required env var: $key" >&2
    exit 1
  fi
done

echo "[1/6] Environment file looks complete: $ENV_FILE"

echo "[2/6] Checking Docker availability"
if command -v docker >/dev/null 2>&1; then
  docker version >/dev/null
else
  echo "Docker is not installed or not on PATH" >&2
  exit 1
fi

if docker compose version >/dev/null 2>&1; then
  COMPOSE=(docker compose)
elif command -v docker-compose >/dev/null 2>&1; then
  COMPOSE=(docker-compose)
else
  echo "Docker Compose is not available" >&2
  exit 1
fi

echo "[3/6] Rendering compose config"
"${COMPOSE[@]}" --env-file "$ENV_FILE" -f "$COMPOSE_FILE" config >/tmp/karyra-spark-compose.rendered.yml

echo "[4/6] Building API image without starting services"
"${COMPOSE[@]}" --env-file "$ENV_FILE" -f "$COMPOSE_FILE" build api

echo "[5/6] Checking migration files"
if compgen -G "migrations/*.sql" >/dev/null; then
  ls migrations/*.sql
else
  echo "No SQL migrations found" >&2
  exit 1
fi

echo "[6/6] Optional live smoke check against $API_BASE_URL"
if command -v curl >/dev/null 2>&1; then
  if curl -fsS "$API_BASE_URL/health/live" >/dev/null 2>&1; then
    echo "API live endpoint responded OK"
  else
    echo "API not running yet, which is acceptable for dry-run. Start compose before runtime smoke tests."
  fi
fi

echo "Server deployment dry-run OK"
