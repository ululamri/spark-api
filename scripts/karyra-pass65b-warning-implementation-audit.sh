#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo "== Pass 65B warning implementation audit =="

grep -RIn '^[[:space:]]*\' src migrations scripts && {
  echo "Unexpected leading backslash found in source/script files." >&2
  exit 1
} || true

grep -q 'fn log_recorded_proof_event' src/proof/ledger.rs
grep -q 'proof_event_id = %event.id' src/proof/ledger.rs
grep -q 'event_hash = event.event_hash.as_deref' src/proof/ledger.rs
grep -q 'evidence_root = event.evidence_root.as_deref' src/proof/ledger.rs
grep -q 'created_at = %event.created_at' src/proof/ledger.rs

grep -q 'DEFAULT_VISIBILITY' src/profile/mod.rs
grep -q 'DEFAULT_AVATAR_PRESET' src/profile/mod.rs
grep -q 'coalesce(profiles.visibility, $2)' src/profile/mod.rs
grep -q 'coalesce(profiles.avatar_preset, $3)' src/profile/mod.rs

echo "Running cargo check with warnings denied..."
RUSTFLAGS="-D warnings" cargo check

echo "Pass 65B warning implementation audit OK"
