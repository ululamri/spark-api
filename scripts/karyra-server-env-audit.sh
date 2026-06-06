#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-config/env.server.example}"
ALLOW_PLACEHOLDERS="${2:-}"

if [[ ! -f "$ENV_FILE" ]]; then
  echo "Env file not found: $ENV_FILE" >&2
  exit 1
fi

required=(
  APP_ENV
  RUST_LOG
  SPARK_API_HOST
  SPARK_API_PORT
  SPARK_WEB_ORIGIN
  POSTGRES_DB
  POSTGRES_USER
  POSTGRES_PASSWORD
  DATABASE_URL
  DATABASE_MAX_CONNECTIONS
  S3_ENDPOINT
  S3_BUCKET_PUBLIC
  S3_BUCKET_PRIVATE
  SPARK_SESSION_COOKIE
  SPARK_SESSION_TTL_DAYS
  SPARK_COOKIE_SECURE
)

get_value() {
  local key="$1"
  grep -E "^${key}=" "$ENV_FILE" | tail -n 1 | cut -d '=' -f 2-
}

for key in "${required[@]}"; do
  if ! grep -qE "^${key}=" "$ENV_FILE"; then
    echo "Missing required env key: $key" >&2
    exit 1
  fi

  value="$(get_value "$key")"
  if [[ -z "${value// }" ]]; then
    echo "Env key is empty: $key" >&2
    exit 1
  fi
done

app_env="$(get_value APP_ENV)"
if [[ "$app_env" != "production" && "$app_env" != "development" ]]; then
  echo "APP_ENV should be production or development, got: $app_env" >&2
  exit 1
fi

port="$(get_value SPARK_API_PORT)"
if ! [[ "$port" =~ ^[0-9]+$ ]]; then
  echo "SPARK_API_PORT must be numeric, got: $port" >&2
  exit 1
fi

max_conn="$(get_value DATABASE_MAX_CONNECTIONS)"
if ! [[ "$max_conn" =~ ^[0-9]+$ ]]; then
  echo "DATABASE_MAX_CONNECTIONS must be numeric, got: $max_conn" >&2
  exit 1
fi

cookie_secure="$(get_value SPARK_COOKIE_SECURE)"
case "${cookie_secure,,}" in
  true|false|1|0|yes|no|on|off) ;;
  *)
    echo "SPARK_COOKIE_SECURE should be boolean-like, got: $cookie_secure" >&2
    exit 1
    ;;
esac

if [[ "$ALLOW_PLACEHOLDERS" != "--allow-placeholders" ]]; then
  if grep -E "replace_with|example\.com|change_me" "$ENV_FILE" >/dev/null; then
    echo "Env file still contains placeholder values. Replace secrets/origins before server use." >&2
    exit 1
  fi
fi

echo "Server env audit OK: $ENV_FILE"
