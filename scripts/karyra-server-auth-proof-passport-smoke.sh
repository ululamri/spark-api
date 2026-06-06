#!/usr/bin/env bash
set -euo pipefail

API_BASE_URL="${1:-http://127.0.0.1:8787}"
API_BASE_URL="${API_BASE_URL%/}"
COOKIE_JAR="$(mktemp)"
BODY_FILE="$(mktemp)"
trap 'rm -f "$COOKIE_JAR" "$BODY_FILE"' EXIT

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "$1 is required" >&2
    exit 1
  fi
}

need curl
need python3

json_post() {
  local path="$1"
  local data="$2"
  local expected_status="${3:-200}"
  local status
  status="$(curl -sS -b "$COOKIE_JAR" -c "$COOKIE_JAR" -H 'Content-Type: application/json' -o "$BODY_FILE" -w "%{http_code}" -X POST "${API_BASE_URL}${path}" --data "$data")"
  if [[ "$status" != "$expected_status" ]]; then
    echo "POST $path failed: expected $expected_status got $status" >&2
    cat "$BODY_FILE" >&2 || true
    exit 1
  fi
  echo "OK POST $path -> $status"
}

json_get() {
  local path="$1"
  local expected_status="${2:-200}"
  local status
  status="$(curl -sS -b "$COOKIE_JAR" -c "$COOKIE_JAR" -o "$BODY_FILE" -w "%{http_code}" "${API_BASE_URL}${path}")"
  if [[ "$status" != "$expected_status" ]]; then
    echo "GET $path failed: expected $expected_status got $status" >&2
    cat "$BODY_FILE" >&2 || true
    exit 1
  fi
  echo "OK GET $path -> $status"
}

assert_json_field_truthy() {
  local field="$1"
  python3 - "$field" "$BODY_FILE" <<'PY'
import json, sys
field, path = sys.argv[1], sys.argv[2]
with open(path, 'r', encoding='utf-8') as f:
    data = json.load(f)
value = data
for part in field.split('.'):
    if isinstance(value, dict):
        value = value.get(part)
    else:
        value = None
        break
if not value:
    print(f"JSON assertion failed: {field} is not truthy", file=sys.stderr)
    print(json.dumps(data, indent=2), file=sys.stderr)
    sys.exit(1)
PY
}

suffix="$(date +%s)-$RANDOM"
email="smoke-${suffix}@example.test"
password="SparkSmokeTest123!"

json_post "/v1/auth/register" "{\"email\":\"${email}\",\"password\":\"${password}\",\"display_name\":\"Smoke Tester\"}" 201
json_get "/v1/auth/me" 200
assert_json_field_truthy "user.id"

json_post "/v1/learning/lessons/intro-starknet/progress" '{"level":"beginner","completed":true,"progress_percent":100}' 200
json_post "/v1/learning/checkpoints/core-basics/results" '{"lesson_id":"intro-starknet","level":"beginner","score":90,"passed":true}' 201
json_post "/v1/learning/exam-attempts" '{"level":"beginner","exam_id":"core-beginner-final","score":92,"passed":true}' 201

json_post "/v1/lab/attempts" '{"lab_id":"wallet-safety-lab","level":"beginner","status":"passed","score":88,"safety_score":95}' 201
json_post "/v1/lab/checkpoints/lab-safety-check/results" '{"lab_id":"wallet-safety-lab","level":"beginner","score":91,"passed":true}' 201
json_post "/v1/lab/exam-attempts" '{"level":"beginner","exam_id":"lab-beginner-final","score":90,"passed":true}' 201

json_get "/v1/proof/me/evidence-root" 200
assert_json_field_truthy "evidence_root"

json_get "/v1/passport/me/eligibility" 200
assert_json_field_truthy "eligible"

json_post "/v1/passport/me/issue" '{"readiness_level":"beginner"}' 201
assert_json_field_truthy "id"
assert_json_field_truthy "credential_hash"

json_post "/v1/auth/logout" '{}' 204

echo "Server auth/proof/passport smoke OK: $API_BASE_URL"
