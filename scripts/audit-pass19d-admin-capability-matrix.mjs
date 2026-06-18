#!/usr/bin/env node
import { readFileSync } from 'node:fs';

const text = readFileSync('src/admin_auth.rs', 'utf8');

const required = [
  'pub fn sanitize_capabilities_for_role',
  'let capabilities = sanitize_capabilities_for_role(&role, &row.capabilities)',
  'pub const MODERATOR_ALLOWED_CAPABILITIES',
  '"ml_moderation_manage",\n            "moderation_read"',
  '"moderation_bulk",\n            "reports_manage"'
];

const forbiddenModeratorAllowed = [
  'pub const MODERATOR_ALLOWED_CAPABILITIES: &[&str] = &[\n    "moderation_read",\n    "moderation_action",\n    "moderation_bulk"',
  'pub const MODERATOR_ALLOWED_CAPABILITIES: &[&str] = &[\n    "moderation_read",\n    "moderation_action",\n    "reports_manage",\n    "content_read",\n    "media_review",\n    "audit_read"'
];

const blockers = [];
for (const item of required) if (!text.includes(item)) blockers.push(`Missing ${item}`);
for (const item of forbiddenModeratorAllowed) if (text.includes(item)) blockers.push(`Forbidden moderator matrix pattern ${item}`);

console.log('PASS 19D admin capability matrix audit');
if (blockers.length) {
  for (const blocker of blockers) console.error(`- ${blocker}`);
  process.exit(1);
}
console.log('No PASS 19D admin capability matrix blockers found.');
