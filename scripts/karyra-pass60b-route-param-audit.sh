#!/usr/bin/env bash
set -euo pipefail

echo "Running Pass 60B backend route param audit..."

grep -q '"/lessons/:lesson_id/progress"' src/learning/mod.rs
grep -q '"/checkpoints/:checkpoint_id/results"' src/learning/mod.rs
grep -q '"/checkpoints/:checkpoint_id/results"' src/lab/mod.rs

if grep -q '"/lessons/{lesson_id}/progress"' src/learning/mod.rs; then
  echo "Old Axum-incompatible lesson route syntax still exists." >&2
  exit 1
fi

if grep -q '"/checkpoints/{checkpoint_id}/results"' src/learning/mod.rs src/lab/mod.rs; then
  echo "Old Axum-incompatible checkpoint route syntax still exists." >&2
  exit 1
fi

echo "Pass 60B backend route param audit OK"
