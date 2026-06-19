#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();

const paths = {
  social: 'src/social/mod.rs',
  adminSocial: 'src/admin_social.rs',
  adminSocialBulk: 'src/admin_social_bulk.rs',
  adminSocialOps: 'src/admin_social_ops.rs',
  adminAuth: 'src/admin_auth.rs'
};

function read(file) {
  const full = path.join(root, file);
  return fs.existsSync(full) ? fs.readFileSync(full, 'utf8') : '';
}

function exists(file) {
  return fs.existsSync(path.join(root, file));
}

function readMigrations() {
  const dir = path.join(root, 'migrations');
  if (!fs.existsSync(dir)) return '';

  return fs
    .readdirSync(dir)
    .filter((file) => file.endsWith('.sql'))
    .sort()
    .map((file) => {
      const full = path.join(dir, file);
      return '\n-- ' + file + '\n' + fs.readFileSync(full, 'utf8');
    })
    .join('\n');
}

function has(text, needle) {
  return needle instanceof RegExp ? needle.test(text) : text.includes(needle);
}

function hasAny(text, needles) {
  return needles.some((needle) => has(text, needle));
}

const files = Object.fromEntries(
  Object.entries(paths).map(([key, file]) => [key, read(file)])
);

const migrations = readMigrations();
const adminModeration = [
  files.adminSocial,
  files.adminSocialBulk,
  files.adminSocialOps
].join('\n');

const all = [
  files.social,
  files.adminSocial,
  files.adminSocialBulk,
  files.adminSocialOps,
  files.adminAuth,
  migrations
].join('\n');

const ok = [];
const warnings = [];
const blockers = [];

function pass(name) {
  ok.push(name);
}

function warn(name) {
  warnings.push(name);
}

function block(name) {
  blockers.push(name);
}

function check(condition, name, level = 'blocker') {
  if (condition) pass(name);
  else if (level === 'warning') warn(name);
  else block(name);
}

console.log('PASS 20D-A audit');
console.log('root: ' + root);
console.log('');

for (const file of Object.values(paths)) {
  check(exists(file), 'file exists: ' + file);
}

check(
  has(migrations, 'social_reports'),
  'schema: social_reports'
);

check(
  has(migrations, 'social_moderation_actions'),
  'schema: social_moderation_actions'
);

check(
  hasAny(files.social, ['social_reports', 'report_post', 'report_comment']) &&
    hasAny(files.social, ['require_current_user', 'current_user']),
  'public reports are user-bound'
);

check(
  hasAny(files.social, ['rate_limit', 'check_rate_limit']),
  'public reports rate limited',
  'warning'
);

check(
  hasAny(files.adminSocial, ['authorize_with_capability', 'admin_auth']),
  'admin social uses shared auth'
);

check(
  hasAny(files.adminSocialBulk, ['authorize_with_capability', 'admin_auth']),
  'bulk moderation uses shared auth'
);

check(
  has(files.adminSocial, 'social_reports') &&
    hasAny(files.adminSocial, ['pending', 'reviewed', 'dismissed', 'status']),
  'admin queue reads report status'
);

check(
  has(adminModeration, 'social_moderation_actions'),
  'moderation actions persisted'
);

check(
  hasAny(adminModeration, ['hide', 'remove', 'restore']) &&
    hasAny(adminModeration, ['social_posts', 'social_comments']) &&
    hasAny(adminModeration, ['hidden', 'removed', 'published']),
  'moderation updates content status'
);

check(
  hasAny(adminModeration, ['mark_reviewed', 'dismiss_report', 'reviewed', 'dismissed']) &&
    has(adminModeration, 'social_reports'),
  'report actions update report state'
);

check(
  hasAny(adminModeration, ['admin_audit_events', 'record_admin_audit', 'audit']),
  'audit trail present',
  'warning'
);

check(
  hasAny(files.adminSocialOps, ['bulk', 'jobs', 'items', 'status']),
  'ops history present',
  'warning'
);

check(
  has(all, 'reports_manage') &&
    has(all, 'moderation_read') &&
    has(all, 'moderation_action'),
  'capabilities referenced'
);

const aggregate = read('scripts/audit-pass20-backend-surface.mjs');

check(
  hasAny(aggregate, [
    'audit-pass20d-admin-social-report-consistency',
    'audit-pass20d'
  ]),
  'aggregate includes pass20d',
  'warning'
);

console.log('OK:');
for (const item of ok) console.log('  OK  ' + item);

if (warnings.length > 0) {
  console.log('');
  console.log('Warnings:');
  for (const item of warnings) console.log('  WARN  ' + item);
}

if (blockers.length > 0) {
  console.log('');
  console.log('Blockers:');
  for (const item of blockers) console.log('  FAIL  ' + item);
  console.log('');
  console.log('PASS 20D-A FAILED');
  process.exit(1);
}

console.log('');
console.log('PASS 20D-A OK');
