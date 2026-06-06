#!/usr/bin/env bash
set -euo pipefail

required=(
  "src/proof/mod.rs"
  "src/proof/ledger.rs"
  "src/learning/mod.rs"
  "src/lab/mod.rs"
  "migrations/000004_proof_event_ledger.sql"
)

for path in "${required[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "Missing required Pass 48 file: $path" >&2
    exit 1
  fi
done

checks=(
  "proof-event-ledger-foundation"
  "record_system_event"
  "SystemProofEventInput"
  "evidence-root"
  "source_table"
  "event_hash"
)

for token in "${checks[@]}"; do
  if ! grep -R "$token" -n src migrations >/dev/null 2>&1; then
    echo "Missing Pass 48 proof ledger token: $token" >&2
    exit 1
  fi
done

echo "Pass 48 proof event ledger audit OK"
