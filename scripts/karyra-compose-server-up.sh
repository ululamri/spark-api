#!/usr/bin/env bash
set -euo pipefail

if [[ ! -f .env.server ]]; then
  echo "Missing .env.server. Create it with:" >&2
  echo "  cp config/env.server.example .env.server" >&2
  echo "Then replace secrets and production URLs." >&2
  exit 1
fi

set -a
# shellcheck disable=SC1091
source .env.server
set +a

docker compose -f infra/docker-compose.server.example.yml --env-file .env.server up -d --build
