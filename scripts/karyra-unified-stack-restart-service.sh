#!/usr/bin/env bash
set -euo pipefail

env_file="${1:-}"
service="${2:-}"
compose_file="infra/docker-compose.unified.staging.yml"

if [[ -z "$env_file" || -z "$service" ]]; then
  echo "Usage: bash scripts/karyra-unified-stack-restart-service.sh config/env.unified.staging spark-api" >&2
  echo "Services: caddy, spark-web, spark-api, postgres, minio" >&2
  exit 1
fi

docker compose --env-file "$env_file" -f "$compose_file" up -d --build "$service"
docker compose --env-file "$env_file" -f "$compose_file" ps "$service"
