#!/usr/bin/env bash
set -euo pipefail

required=(
  "Dockerfile"
  "config/env.server.example"
  "config/release.env.example"
  "infra/docker-compose.server.example.yml"
  "scripts/karyra-server-deploy-dry-run.sh"
  "scripts/karyra-server-backup-postgres.sh"
  "scripts/karyra-server-restore-postgres.sh"
  "scripts/karyra-server-minio-readiness.sh"
  "scripts/karyra-server-release-summary.sh"
  "docs/server/DEPLOYMENT_DRY_RUN.md"
  "docs/server/BACKUP_ROLLBACK_RUNBOOK.md"
  "docs/server/RELEASE_CHECKLIST.md"
)

for path in "${required[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "Missing required Pass 53 file: $path" >&2
    exit 1
  fi
done

for script in scripts/karyra-server-*.sh scripts/karyra-pass53-server-deployment-dry-run-audit.sh; do
  if [[ ! -x "$script" ]]; then
    echo "Script is not executable: $script" >&2
    exit 1
  fi
done

if grep -R "replace_with_strong_password" -n config/release.env.example >/dev/null; then
  :
else
  echo "config/release.env.example should keep placeholder secret values" >&2
  exit 1
fi

echo "Pass 53 server deployment dry-run audit OK"
