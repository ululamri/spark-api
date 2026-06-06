#!/usr/bin/env bash
set -euo pipefail

required_paths=(
  "infra/caddy/Caddyfile"
  "infra/docker-compose.unified.staging.yml"
  "config/env.unified.staging.example"
  "scripts/karyra-unified-stack-deploy.sh"
)

for path in "${required_paths[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "Missing required file: $path" >&2
    exit 1
  fi
done

if ! grep -q "auto_https off" infra/caddy/Caddyfile; then
  echo "Caddyfile must set auto_https off for provider-managed HTTPS mode" >&2
  exit 1
fi

if ! grep -q 'http://{$SPARK_DOMAIN}' infra/caddy/Caddyfile; then
  echo 'Caddyfile must use http://{$SPARK_DOMAIN} in provider-managed HTTPS mode' >&2
  exit 1
fi

if grep -q 'https://{$SPARK_DOMAIN}' infra/caddy/Caddyfile; then
  echo 'Caddyfile should not use https://{$SPARK_DOMAIN} in provider-managed HTTPS mode' >&2
  exit 1
fi

if ! grep -q "SPARK_WEB_ORIGIN=https://" config/env.unified.staging.example; then
  echo "env example must keep public SPARK_WEB_ORIGIN as https://..." >&2
  exit 1
fi

if ! grep -q "SPARK_HTTPS_MODE=provider-managed" config/env.unified.staging.example; then
  echo "env example should document SPARK_HTTPS_MODE=provider-managed" >&2
  exit 1
fi

echo "Pass 56 provider-managed HTTPS audit OK"
