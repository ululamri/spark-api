#!/usr/bin/env bash
set -euo pipefail

env_file="${1:-config/env.unified.staging}"
service="${2:-}"
compose_file="infra/docker-compose.unified.staging.yml"

if [[ -n "$service" ]]; then
  docker compose --env-file "$env_file" -f "$compose_file" logs -f --tail=150 "$service"
else
  docker compose --env-file "$env_file" -f "$compose_file" logs -f --tail=80
fi
