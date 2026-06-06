#!/usr/bin/env bash
set -euo pipefail

env_file="${1:-config/env.unified.staging}"

if [[ ! -f "$env_file" ]]; then
  echo "Env file not found: $env_file" >&2
  echo "Create it from config/env.unified.staging.example first." >&2
  exit 1
fi

required_keys=(
  "SPARK_DOMAIN"
  "SPARK_WEB_ORIGIN"
  "POSTGRES_DB"
  "POSTGRES_USER"
  "POSTGRES_PASSWORD"
  "DATABASE_URL"
  "S3_ENDPOINT"
  "S3_BUCKET_PUBLIC"
  "S3_BUCKET_PRIVATE"
  "MINIO_ROOT_USER"
  "MINIO_ROOT_PASSWORD"
  "SPARK_COOKIE_SECURE"
)

for key in "${required_keys[@]}"; do
  if ! grep -Eq "^${key}=" "$env_file"; then
    echo "Missing required env key: $key" >&2
    exit 1
  fi
done

if grep -Eq "CHANGE_ME|replace_with|example.com" "$env_file"; then
  echo "Env file still contains placeholder values. Replace them before deploy." >&2
  exit 1
fi

source "$env_file"

if [[ "${SPARK_WEB_ORIGIN:-}" != "https://${SPARK_DOMAIN}" ]]; then
  echo "SPARK_WEB_ORIGIN should usually equal https://SPARK_DOMAIN for same-server stack." >&2
  echo "SPARK_WEB_ORIGIN=${SPARK_WEB_ORIGIN:-}" >&2
  echo "SPARK_DOMAIN=${SPARK_DOMAIN:-}" >&2
  exit 1
fi

if [[ "${SPARK_COOKIE_SECURE:-}" != "true" ]]; then
  echo "SPARK_COOKIE_SECURE should be true when serving HTTPS." >&2
  exit 1
fi

echo "Unified stack env audit OK: $env_file"
