#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${SPARK_API_BASE_URL:-http://127.0.0.1:8787}"

require_curl() {
  if ! command -v curl >/dev/null 2>&1; then
    echo "curl is required" >&2
    exit 1
  fi
}

check_get() {
  local path="$1"
  echo "GET ${BASE_URL}${path}"
  curl -fsS "${BASE_URL}${path}" >/dev/null
}

require_curl

check_get "/"
check_get "/health/live"
check_get "/v1/auth/scope"
check_get "/v1/learning/scope"
check_get "/v1/lab/scope"
check_get "/v1/proof/scope"
check_get "/v1/passport/scope"
check_get "/v1/media/policy"

if [[ "${CHECK_READY:-0}" == "1" ]]; then
  check_get "/health/ready"
fi

echo "Spark API smoke test OK"
