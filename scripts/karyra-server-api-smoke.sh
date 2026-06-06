#!/usr/bin/env bash
set -euo pipefail

API_BASE_URL="${1:-http://127.0.0.1:8787}"
API_BASE_URL="${API_BASE_URL%/}"

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "$1 is required" >&2
    exit 1
  fi
}

need curl

check_get() {
  local path="$1"
  local expected_status="${2:-200}"
  local url="${API_BASE_URL}${path}"
  local status
  status="$(curl -sS -o /tmp/spark-api-smoke-response.json -w "%{http_code}" "$url")"
  if [[ "$status" != "$expected_status" ]]; then
    echo "Smoke GET failed: $path expected $expected_status got $status" >&2
    cat /tmp/spark-api-smoke-response.json >&2 || true
    exit 1
  fi
  echo "OK GET $path -> $status"
}

check_get "/" 200
check_get "/health/live" 200
check_get "/health/ready" 200
check_get "/v1/auth/scope" 200
check_get "/v1/learning/scope" 200
check_get "/v1/lab/scope" 200
check_get "/v1/proof/scope" 200
check_get "/v1/passport/scope" 200
check_get "/v1/media/policy" 200

echo "Server API smoke OK: $API_BASE_URL"
