#!/usr/bin/env bash
set -euo pipefail

required=(
  "Dockerfile"
  "config/env.server.example"
  "infra/docker-compose.server.example.yml"
  "scripts/karyra-server-env-audit.sh"
  "scripts/karyra-server-migration-check.sh"
  "scripts/karyra-server-api-smoke.sh"
  "scripts/karyra-server-auth-proof-passport-smoke.sh"
  "docs/server/PASS_52_SERVER_RUNTIME_HARDENING.md"
  "docs/server/SERVER_RUNTIME_CHECKLIST.md"
)

for path in "${required[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "Missing required Pass 52 file: $path" >&2
    exit 1
  fi
done

if ! grep -q "USER spark" Dockerfile; then
  echo "Dockerfile should run the API as the non-root spark user" >&2
  exit 1
fi

if ! grep -q "HEALTHCHECK" Dockerfile; then
  echo "Dockerfile should include a healthcheck" >&2
  exit 1
fi

for key in APP_ENV SPARK_API_HOST SPARK_API_PORT SPARK_WEB_ORIGIN DATABASE_URL S3_ENDPOINT SPARK_COOKIE_SECURE; do
  if ! grep -q "^${key}=" config/env.server.example; then
    echo "Missing server env key in config/env.server.example: $key" >&2
    exit 1
  fi
done

for service in "api:" "postgres:" "minio:"; do
  if ! grep -q "$service" infra/docker-compose.server.example.yml; then
    echo "Missing service in infra/docker-compose.server.example.yml: $service" >&2
    exit 1
  fi
done

bash scripts/karyra-server-env-audit.sh config/env.server.example --allow-placeholders

echo "Pass 52 server runtime hardening audit OK"
