#!/usr/bin/env bash
set -euo pipefail

base_url="${1:-}"
if [[ -z "$base_url" ]]; then
  echo "Usage: bash scripts/karyra-unified-stack-smoke.sh https://spark.user.cloudjkt01.com" >&2
  exit 1
fi

base_url="${base_url%/}"

check() {
  local path="$1"
  local url="${base_url}${path}"
  echo "Checking $url"
  curl -fsS "$url" >/tmp/karyra-smoke-response.json
  head -c 300 /tmp/karyra-smoke-response.json || true
  echo
}

check "/health/live"
check "/health/ready"
check "/v1/auth/scope"
check "/v1/learning/scope"
check "/v1/lab/scope"
check "/v1/proof/scope"
check "/v1/passport/scope"
check "/v1/media/policy"

# Frontend check: non-fatal because route rendering can differ during early staging.
echo "Checking frontend root ${base_url}/"
if curl -fsS "${base_url}/" >/tmp/karyra-frontend-smoke.html; then
  echo "Frontend root OK"
else
  echo "Frontend root check failed; inspect spark-web and caddy logs." >&2
  exit 1
fi

echo "Unified stack smoke test OK"
