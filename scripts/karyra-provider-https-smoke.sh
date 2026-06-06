#!/usr/bin/env bash
set -euo pipefail

input="${1:-spark.user.cloudjkt01.com}"

if [[ "$input" == http://* || "$input" == https://* ]]; then
  public_base="${input%/}"
  echo "Running public URL smoke test: $public_base"
  curl -fsS "$public_base/health/live" >/dev/null
  curl -fsS "$public_base/v1/auth/scope" >/dev/null
  curl -fsSI "$public_base/" >/dev/null
else
  domain="$input"
  echo "Running provider-managed internal smoke test for domain: $domain"
  curl -fsS -H "Host: $domain" "http://127.0.0.1/health/live" >/dev/null
  curl -fsS -H "Host: $domain" "http://127.0.0.1/v1/auth/scope" >/dev/null
  curl -fsSI -H "Host: $domain" "http://127.0.0.1/" >/dev/null
fi

echo "Provider-managed HTTPS smoke test OK"
