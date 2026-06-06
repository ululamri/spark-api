#!/usr/bin/env bash
set -euo pipefail

required=(
  "src/media/mod.rs"
  "migrations/000006_media_upload_foundation.sql"
  "scripts/karyra-pass50-media-upload-audit.sh"
)

for path in "${required[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "Missing required Pass 50 file: $path" >&2
    exit 1
  fi
done

grep -q 'route("/upload-intents"' src/media/mod.rs
grep -q 'route("/me/assets"' src/media/mod.rs
grep -q 'route("/assets/{asset_id}/complete"' src/media/mod.rs
grep -q 'route("/assets/{asset_id}/links"' src/media/mod.rs
grep -q 'require_current_user' src/media/mod.rs
grep -q 'media_assets' src/media/mod.rs
grep -q 'media_links' src/media/mod.rs
grep -q 'original_file_name' migrations/000006_media_upload_foundation.sql
grep -q 'upload_expires_at' migrations/000006_media_upload_foundation.sql
grep -q 'idx_media_links_unique_asset_entity_purpose' migrations/000006_media_upload_foundation.sql

echo "Pass 50 media upload foundation audit OK"
