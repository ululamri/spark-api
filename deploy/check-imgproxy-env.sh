#!/usr/bin/env sh
set -eu

ENV_FILE="${1:-/etc/karyra/imgproxy.env}"

if [ ! -f "$ENV_FILE" ]; then
  echo "imgproxy env file not found: $ENV_FILE" >&2
  exit 1
fi

missing=0
for key in IMGPROXY_KEY IMGPROXY_SALT IMGPROXY_BIND; do
  if ! grep -q "^${key}=" "$ENV_FILE"; then
    echo "missing ${key}" >&2
    missing=1
  fi
done

if [ "$missing" -ne 0 ]; then
  exit 1
fi

key_len=$(grep '^IMGPROXY_KEY=' "$ENV_FILE" | head -1 | cut -d= -f2- | tr -d '[:space:]' | wc -c | tr -d ' ')
salt_len=$(grep '^IMGPROXY_SALT=' "$ENV_FILE" | head -1 | cut -d= -f2- | tr -d '[:space:]' | wc -c | tr -d ' ')

if [ "$key_len" -lt 64 ]; then
  echo "IMGPROXY_KEY should be at least 64 hex characters" >&2
  exit 1
fi

if [ "$salt_len" -lt 64 ]; then
  echo "IMGPROXY_SALT should be at least 64 hex characters" >&2
  exit 1
fi

echo "imgproxy env looks ready: $ENV_FILE"
