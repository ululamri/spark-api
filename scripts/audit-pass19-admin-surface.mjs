#!/usr/bin/env node
import { spawnSync } from 'node:child_process';

const scripts = [
  'scripts/audit-pass19c-admin-auth.mjs',
  'scripts/audit-pass19c-admin-team.mjs',
  'scripts/audit-pass19d-admin-capability-matrix.mjs',
  'scripts/audit-pass19d-admin-team-effective-capabilities.mjs',
  'scripts/audit-pass19e-admin-backend-surface.mjs'
];

let failed = false;
console.log('PASS 19 backend admin surface aggregate audit');
for (const script of scripts) {
  console.log(`\n> node ${script}`);
  const result = spawnSync(process.execPath, [script], { stdio: 'inherit' });
  if (result.status !== 0) failed = true;
}

if (failed) process.exit(1);
console.log('\nNo PASS 19 backend admin surface blockers found.');
