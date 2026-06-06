#!/usr/bin/env bash
set -euo pipefail

required=(
  "src/auth/session.rs"
  "src/progress.rs"
  "src/learning/mod.rs"
  "src/lab/mod.rs"
  "src/http/mod.rs"
  "migrations/000003_learning_lab_progress.sql"
)

for path in "${required[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "Missing required Pass 47 file: $path" >&2
    exit 1
  fi
done

if ! grep -q "pub mod session;" src/auth/mod.rs; then
  echo "src/auth/mod.rs must expose auth::session" >&2
  exit 1
fi

if ! grep -q "mod progress;" src/main.rs; then
  echo "src/main.rs must register progress module" >&2
  exit 1
fi

if ! grep -q "lesson_progress" migrations/000003_learning_lab_progress.sql; then
  echo "Pass 47 migration must define lesson_progress" >&2
  exit 1
fi

if ! grep -q "lab_attempts" migrations/000003_learning_lab_progress.sql; then
  echo "Pass 47 migration must define lab_attempts" >&2
  exit 1
fi

if ! grep -q "checkpoint_results" migrations/000003_learning_lab_progress.sql; then
  echo "Pass 47 migration must define checkpoint_results" >&2
  exit 1
fi

if ! grep -q 'lessons/{lesson_id}/progress' src/learning/mod.rs; then
  echo "Learning progress endpoint route is missing" >&2
  exit 1
fi

if ! grep -q 'checkpoints/{checkpoint_id}/results' src/lab/mod.rs; then
  echo "Lab checkpoint result endpoint route is missing" >&2
  exit 1
fi

echo "Pass 47 learning/lab progress API audit OK"
