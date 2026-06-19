#!/usr/bin/env node
import { spawnSync } from 'node:child_process';

const scripts = [
  'scripts/audit-pass19-admin-surface.mjs',
  'scripts/audit-pass20a-social-media-surface.mjs'
];

let failed = false;
console.log('PASS 20 backend surface aggregate audit');
for (const script of scripts) {
  console.log(`\n> node ${script}`);
  const result = spawnSync(process.execPath, [script], { stdio: 'inherit' });
  if (result.status !== 0) failed = true;
}

if (failed) process.exit(1);
console.log('\nNo PASS 20 backend surface blockers found.');
