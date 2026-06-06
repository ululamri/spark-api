#!/usr/bin/env bash
set -euo pipefail

required=(
  "infra/docker-compose.unified.staging.yml"
  "config/env.unified.staging.example"
  "scripts/karyra-unified-stack-deploy.sh"
)

for path in "${required[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "Missing required backend stack file: $path" >&2
    exit 1
  fi
done

if ! grep -q "PUBLIC_API_BASE" infra/docker-compose.unified.staging.yml; then
  echo "Unified compose is missing frontend PUBLIC_API_BASE build arg" >&2
  exit 1
fi

if ! grep -q "SPARK_WEB_RUNTIME_ENV_FILE" infra/docker-compose.unified.staging.yml; then
  echo "Unified compose is missing frontend runtime env file support" >&2
  exit 1
fi

if ! grep -q "PUBLIC_SPARK_API_BASE" config/env.unified.staging.example; then
  echo "Unified env example is missing frontend public API env" >&2
  exit 1
fi

echo "Pass 55 unified stack audit OK"
