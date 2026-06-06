#!/usr/bin/env bash
set -euo pipefail

env_file="${1:-config/env.unified.staging}"
compose_file="infra/docker-compose.unified.staging.yml"

bash scripts/karyra-unified-stack-env-audit.sh "$env_file"

if ! command -v docker >/dev/null 2>&1; then
  echo "Docker is not installed or not in PATH" >&2
  exit 1
fi

if ! docker compose version >/dev/null 2>&1; then
  echo "Docker Compose plugin is not available" >&2
  exit 1
fi

echo "Building and starting unified Spark stack..."
docker compose --env-file "$env_file" -f "$compose_file" up -d --build

echo "Stack status:"
docker compose --env-file "$env_file" -f "$compose_file" ps
