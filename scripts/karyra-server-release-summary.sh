#!/usr/bin/env bash
set -euo pipefail

API_BASE_URL="${1:-http://127.0.0.1:8787}"

echo "Karyra Spark API release summary"
echo "================================"
echo "Git commit: $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "Git branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
echo "API base:   $API_BASE_URL"
echo

echo "Local files:"
for path in Dockerfile infra/docker-compose.server.example.yml config/env.server.example config/release.env.example; do
  if [[ -f "$path" ]]; then
    echo "  OK  $path"
  else
    echo "  MISS $path"
  fi
done

echo
if command -v curl >/dev/null 2>&1; then
  echo "API live:"
  curl -fsS "$API_BASE_URL/health/live" || true
  echo
  echo "API root:"
  curl -fsS "$API_BASE_URL/" || true
  echo
fi
