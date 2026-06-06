#!/usr/bin/env bash
set -euo pipefail

backend_required=(
  "Dockerfile"
  "infra/docker-compose.unified.staging.yml"
  "infra/caddy/Caddyfile"
  "config/env.unified.staging.example"
  "scripts/karyra-unified-stack-env-audit.sh"
  "scripts/karyra-unified-stack-deploy.sh"
  "scripts/karyra-unified-stack-smoke.sh"
  "docs/PASS_54_UNIFIED_DOCKER_STACK.md"
)

for path in "${backend_required[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "Missing required Pass 54 backend file: $path" >&2
    exit 1
  fi
done

if ! grep -q "spark-web" infra/docker-compose.unified.staging.yml; then
  echo "Unified compose is missing spark-web service" >&2
  exit 1
fi

if ! grep -q "spark-api" infra/docker-compose.unified.staging.yml; then
  echo "Unified compose is missing spark-api service" >&2
  exit 1
fi

if ! grep -q "postgres" infra/docker-compose.unified.staging.yml; then
  echo "Unified compose is missing postgres service" >&2
  exit 1
fi

if ! grep -q "minio" infra/docker-compose.unified.staging.yml; then
  echo "Unified compose is missing minio service" >&2
  exit 1
fi

if ! grep -q 'reverse_proxy @api spark-api:8787' infra/caddy/Caddyfile; then
  echo "Caddyfile is missing API reverse proxy" >&2
  exit 1
fi

frontend_root="${SPARK_FRONTEND_ROOT:-../spark}"
if [[ -d "$frontend_root" ]]; then
  if [[ ! -f "$frontend_root/Dockerfile.staging" ]]; then
    echo "Missing frontend Dockerfile.staging in $frontend_root" >&2
    exit 1
  fi
else
  echo "Note: frontend root not found at $frontend_root; skipping frontend file audit."
fi

echo "Pass 54 unified Docker stack audit OK"
