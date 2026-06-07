\
#!/usr/bin/env bash
set -euo pipefail

required_files=(
  "src/community/mod.rs"
  "src/hub/mod.rs"
  "migrations/0064_community_hub_signals.sql"
)

for file in "${required_files[@]}"; do
  [[ -f "$file" ]] || { echo "Missing $file" >&2; exit 1; }
done

grep -q 'workshops/:workshop_id/register' src/community/mod.rs
grep -q 'proof_of_participation_signal_recorded' src/community/mod.rs
grep -q 'resources/:resource_id/save' src/hub/mod.rs
grep -q 'proof_of_exploration_signal_recorded' src/hub/mod.rs
grep -q 'community_workshop_registrations' migrations/0064_community_hub_signals.sql
grep -q 'hub_resource_saves' migrations/0064_community_hub_signals.sql

echo "Pass 64 backend community/hub signals audit OK"
