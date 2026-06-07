#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "[pass65][backend][FAIL] $*" >&2
  exit 1
}

warn() {
  echo "[pass65][backend][WARN] $*" >&2
}

info() {
  echo "[pass65][backend] $*"
}

[[ -f Cargo.toml ]] || fail "Run this script from the spark-api root."
[[ -d src ]] || fail "Missing src directory."

info "Checking module registration..."
for module in auth learning lab proof passport profile media community hub; do
  [[ -d "src/${module}" || -f "src/${module}.rs" ]] || fail "Missing src/${module} module."
  grep -R "nest(\"/v1/${module}\"" -n src/http src/http.rs 2>/dev/null >/dev/null || fail "Missing /v1/${module} router nest."
done

info "Checking route boundary scopes..."
for module in auth learning lab proof passport profile media community hub; do
  grep -R "route(\"/scope\"" -n "src/${module}" 2>/dev/null >/dev/null || warn "No /scope route found in ${module}."
done

info "Checking accidental leading backslash introduced in code files..."
if grep -RIn --include='*.rs' --include='*.sql' --include='*.sh' '^[[:space:]]*\\' src migrations scripts 2>/dev/null; then
  fail "Found suspicious leading backslash in backend source/migration/script files."
fi

info "Checking Axum 0.7 route parameter syntax..."
if grep -RIn --include='*.rs' 'route("[^"]*{[A-Za-z0-9_][A-Za-z0-9_]*}' src 2>/dev/null; then
  fail "Found Axum 0.8-style {param} route syntax. Use :param for Axum 0.7 routes."
fi

info "Checking expected dynamic route params..."
grep -R 'route("/lessons/:lesson_id/progress"' -n src/learning >/dev/null || fail "Missing learning lesson progress :lesson_id route."
grep -R 'route("/checkpoints/:checkpoint_id/results"' -n src/learning >/dev/null || fail "Missing learning checkpoint :checkpoint_id route."
grep -R 'route("/checkpoints/:checkpoint_id/results"' -n src/lab >/dev/null || fail "Missing lab checkpoint :checkpoint_id route."
grep -R 'route("/assets/:asset_id"' -n src/media >/dev/null || fail "Missing media asset :asset_id route."
grep -R 'route("/assets/:asset_id/complete"' -n src/media >/dev/null || fail "Missing media asset complete :asset_id route."
grep -R 'route("/assets/:asset_id/links"' -n src/media >/dev/null || fail "Missing media asset links :asset_id route."

grep -R 'route("/workshops/:workshop_id/register"' -n src/community >/dev/null || fail "Missing community workshop register :workshop_id route."
grep -R 'route("/resources/:resource_id/save"' -n src/hub >/dev/null || fail "Missing hub resource save :resource_id route."

info "Checking proof event integration markers..."
for marker in \
  proof_of_learning_lesson_completed \
  proof_of_practice_lab_attempt_recorded \
  proof_of_safety_score_recorded \
  proof_of_participation_signal_recorded \
  proof_of_exploration_signal_recorded; do
  grep -R "$marker" -n src >/dev/null || fail "Missing proof marker: $marker"
done

info "Checking migration idempotency surface..."
if [[ -d migrations ]]; then
  if grep -RInE '^[[:space:]]*create[[:space:]]+table[[:space:]]+[^i]' migrations 2>/dev/null; then
    warn "Some migration CREATE TABLE lines may not use IF NOT EXISTS. Review if re-running all migrations manually."
  fi
  if grep -RInE '^[[:space:]]*create[[:space:]]+index[[:space:]]+[^i]' migrations 2>/dev/null; then
    warn "Some migration CREATE INDEX lines may not use IF NOT EXISTS. Review if re-running all migrations manually."
  fi
else
  warn "No migrations directory found."
fi

info "Checking public API root mentions current backend capabilities..."
grep -R "authenticated Core/Learn and Lab progress" -n src/http src/http.rs 2>/dev/null >/dev/null || warn "API root phase text may be stale."

info "Pass 65 backend integration seal audit OK"
