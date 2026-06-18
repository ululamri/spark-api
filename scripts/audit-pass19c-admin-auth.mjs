#!/usr/bin/env node
import { readFileSync } from 'node:fs';

const text = readFileSync('src/admin_auth.rs', 'utf8');
const required = [
  'pub async fn authorize_admin_actor',
  'authorize_super_admin_token',
  'require_current_user',
  'expires_at is null or expires_at > now()',
  'starts_at <= now()',
  'pub async fn authorize_with_capability',
  'context.capabilities.iter().any'
];

const blockers = required.filter((item) => !text.includes(item));
console.log('PASS 19C admin auth audit');
if (blockers.length) {
  for (const item of blockers) console.error(`Missing ${item}`);
  process.exit(1);
}
console.log('No PASS 19C admin auth blockers found.');
