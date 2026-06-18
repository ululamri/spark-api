#!/usr/bin/env node
import { readFileSync } from 'node:fs';

const text = readFileSync('src/admin_team.rs', 'utf8');
const required = [
  'admin-rbac-effective-status',
  '.route("/actor", get(actor))',
  'GET /api/admin/team/actor',
  'authorize_admin_actor',
  'with assignments as',
  'expires_at <= now()',
  'expires_at > now()',
  'expires_at must be in the future',
  "'active' as status",
  'starts_at <= now()'
];

const blockers = required.filter((item) => !text.includes(item));
console.log('PASS 19C admin team audit');
if (blockers.length) {
  for (const item of blockers) console.error(`Missing ${item}`);
  process.exit(1);
}
console.log('No PASS 19C admin team blockers found.');
