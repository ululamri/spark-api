#!/usr/bin/env bash
set -euo pipefail

required=(
  "src/passport/mod.rs"
  "migrations/000005_passport_credential_api.sql"
)

for path in "${required[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "Missing required Pass 49 file: $path" >&2
    exit 1
  fi
done

grep -R "route(\"/me/eligibility\"" -n src/passport/mod.rs >/dev/null
grep -R "route(\"/me/issue\"" -n src/passport/mod.rs >/dev/null
grep -R "route(\"/me/revoke\"" -n src/passport/mod.rs >/dev/null
grep -R "system proof records only" -n src/passport/mod.rs >/dev/null
grep -R "starknet_anchor_status" -n src/passport/mod.rs >/dev/null
grep -R "idx_passport_credentials_user_issued_unique" -n migrations/000005_passport_credential_api.sql >/dev/null

if grep -R "mint" -n src/passport/mod.rs | grep -E "route|post|get"; then
  echo "Pass 49 must not add NFT mint routes before grant scope" >&2
  exit 1
fi

echo "Pass 49 passport credential API audit OK"
