#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if grep -R 'assets/{asset_id}' -n src/media/mod.rs >/dev/null; then
  echo "Axum 0.7 route param bug still present in src/media/mod.rs" >&2
  exit 1
fi

for pattern in '"/assets/:asset_id"' '"/assets/:asset_id/complete"' '"/assets/:asset_id/links"'; do
  if ! grep -F "$pattern" src/media/mod.rs >/dev/null; then
    echo "Missing expected media route pattern: $pattern" >&2
    exit 1
  fi
done

if ! grep -F '.route("/upload-intents", post(create_upload_intent))' src/media/mod.rs >/dev/null; then
  echo "Missing media upload intent route" >&2
  exit 1
fi

if ! grep -F '.nest("/v1/media", crate::media::router())' src/http/mod.rs >/dev/null; then
  echo "Media router is not mounted at /v1/media" >&2
  exit 1
fi

echo "Pass 63 backend media runtime audit OK"
