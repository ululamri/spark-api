#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();

const files = {
  doc: 'docs/ADMIN_MODERATION_REPORT_LIFECYCLE_SMOKE.md',
  social: 'src/social/mod.rs',
  adminSocial: 'src/admin_social.rs',
  adminSocialBulk: 'src/admin_social_bulk.rs',
  adminSocialOps: 'src/admin_social_ops.rs',
  adminAudit: 'src/admin_audit.rs',
  adminAuth: 'src/admin_auth.rs'
};

function exists(file) {
  return fs.existsSync(path.join(root, file));
}

function read(file) {
  const full = path.join(root, file);
  return fs.existsSync(full) ? fs.readFileSync(full, 'utf8') : '';
}

function has(text, needle) {
  return needle instanceof RegExp ? needle.test(text) : text.includes(needle);
}

function hasAny(text, needles) {
  return needles.some((needle) => has(text, needle));
}

const src = Object.fromEntries(
  Object.entries(files).map(([key, file]) => [key, read(file)])
);

const all = Object.values(src).join('\n');

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

console.log('PASS 20F-A report lifecycle smoke readiness audit');
console.log('root: ' + root);
console.log('');

for (const file of Object.values(files)) {
  check(exists(file), 'file exists: ' + file);
}

check(
  hasAny(src.doc, [
    'social report',
    'admin moderation queue',
    'moderation action',
    'audit trail'
  ]),
  'smoke doc describes full report lifecycle'
);

check(
  hasAny(src.doc, [
    'no demo seed',
    'no fake data',
    'Do not insert'
  ]),
  'smoke doc forbids direct fake/demo seed data'
);

check(
  hasAny(src.social, [
    '/report',
    'report_post',
    'report_comment'
  ]) &&
    has(src.social, 'social_reports'),
  'public social report creation route exists'
);

check(
  hasAny(src.social, [
    'require_current_user',
    'current_user'
  ]),
  'public report creation requires user context'
);

check(
  hasAny(src.adminSocial, [
    'social_reports',
    'reports'
  ]) &&
    hasAny(src.adminSocial, [
      'pending',
      'reviewed',
      'dismissed',
      'status'
    ]),
  'admin social queue reads report states'
);

check(
  hasAny(src.adminSocial + src.adminSocialBulk, [
    'mark_reviewed',
    'dismiss_report',
    'reviewed',
    'dismissed'
  ]),
  'report review/dismiss actions exist'
);

check(
  hasAny(src.adminSocial + src.adminSocialBulk, [
    'hide',
    'remove',
    'restore'
  ]) &&
    hasAny(src.adminSocial + src.adminSocialBulk, [
      'social_posts',
      'social_comments'
    ]),
  'content hide/remove/restore actions exist'
);

check(
  has(src.adminSocialBulk, 'dry_run') &&
    hasAny(src.adminSocialBulk, [
      'would_apply_count',
      'applied_count'
    ]),
  'bulk moderation dry-run counters exist'
);

check(
  hasAny(src.adminSocialOps, [
    'bulk-jobs',
    'bulkJobs',
    'job_id',
    'items'
  ]),
  'bulk job history/readback exists'
);

check(
  hasAny(src.adminAudit, [
    'admin_audit_events',
    'audit_read',
    'events'
  ]) ||
    hasAny(all, [
      'admin_audit_events',
      'record_admin_audit'
    ]),
  'admin audit trail exists'
);

check(
  hasAny(all, [
    'moderation_read',
    'moderation_action',
    'moderation_restore',
    'moderation_bulk',
    'reports_manage',
    'audit_read'
  ]),
  'required capabilities are referenced'
);

check(
  hasAny(src.adminAuth, [
    'authorize_with_capability',
    'authorize_super_admin_only',
    'authorize_admin_actor'
  ]),
  'shared admin auth is available'
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
  console.log('PASS 20F-A FAILED');
  process.exit(1);
}

console.log('');
console.log('PASS 20F-A OK');
