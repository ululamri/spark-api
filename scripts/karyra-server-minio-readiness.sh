#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-.env.release}"

if [[ ! -f "$ENV_FILE" ]]; then
  echo "Missing env file: $ENV_FILE" >&2
  exit 1
fi

set -a
# shellcheck source=/dev/null
source "$ENV_FILE"
set +a

: "${S3_ENDPOINT:?S3_ENDPOINT is required}"
: "${S3_BUCKET_PUBLIC:?S3_BUCKET_PUBLIC is required}"
: "${S3_BUCKET_PRIVATE:?S3_BUCKET_PRIVATE is required}"
: "${MINIO_ROOT_USER:?MINIO_ROOT_USER is required}"
: "${MINIO_ROOT_PASSWORD:?MINIO_ROOT_PASSWORD is required}"

if ! command -v docker >/dev/null 2>&1; then
  echo "Docker is required for this MinIO readiness helper unless mc is installed manually." >&2
  exit 1
fi

MC_ALIAS="spark"
MC_IMAGE="minio/mc:latest"

docker run --rm --network host "$MC_IMAGE" sh -c "
  mc alias set $MC_ALIAS '$S3_ENDPOINT' '$MINIO_ROOT_USER' '$MINIO_ROOT_PASSWORD' >/dev/null &&
  mc mb --ignore-existing $MC_ALIAS/$S3_BUCKET_PUBLIC &&
  mc mb --ignore-existing $MC_ALIAS/$S3_BUCKET_PRIVATE &&
  mc ls $MC_ALIAS
"

echo "MinIO/S3-compatible bucket readiness OK"
