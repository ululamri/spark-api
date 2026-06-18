#!/usr/bin/env node
import { readFileSync } from 'node:fs';

const text = readFileSync('src/admin_team.rs', 'utf8');
const required = [
  'phase: "admin-rbac-final-matrix"',
  'for item in &mut items',
  'item.capabilities = admin_auth::sanitize_capabilities_for_role(&item.role, &item.capabilities)',
  'let mut assignment = sqlx::query_as',
  'assignment.capabilities =',
  'Review-focused moderation role'
];

const blockers = required.filter((item) => !text.includes(item));
console.log('PASS 19D admin team effective capabilities audit');
if (blockers.length) {
  for (const item of blockers) console.error(`Missing ${item}`);
  process.exit(1);
}
console.log('No PASS 19D admin team effective capability blockers found.');
