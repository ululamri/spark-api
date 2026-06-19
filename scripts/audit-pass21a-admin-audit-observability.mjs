#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();

const files = {
  doc: 'docs/ADMIN_AUDIT_OBSERVABILITY_RUNBOOK.md',
  adminAudit: 'src/admin_audit.rs',
  adminAuth: 'src/admin_auth.rs',
  adminSocial: 'src/admin_social.rs',
  adminSocialBulk: 'src/admin_social_bulk.rs'
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

console.log('PASS 21A admin audit observability audit');
console.log('root: ' + root);
console.log('');

for (const file of Object.values(files)) {
  check(exists(file), 'file exists: ' + file);
}

check(
  hasAny(src.doc, [
    'Admin Audit Trail',
    'admin_audit_events',
    'audit_read'
  ]),
  'runbook documents audit observability model'
);

check(
  has(src.adminAudit, '.route("/scope", get(scope))') &&
    has(src.adminAudit, '.route("/events", get(events))') &&
    has(src.adminAudit, '.route("/events/:event_id", get(event_detail))'),
  'audit API exposes scope/list/detail routes'
);

check(
  (src.adminAudit.match(/authorize_with_capability\(&state, &headers, "audit_read"\)/g) || []).length >= 3,
  'audit API enforces audit_read on scope/list/detail'
);

check(
  has(src.adminAudit, 'limit.unwrap_or(50).clamp(1, 100)') &&
    has(src.adminAudit, 'offset.unwrap_or(0).max(0)'),
  'audit list has paging guardrails'
);

check(
  has(src.adminAudit, 'actor_kind: Option<String>') &&
    has(src.adminAudit, 'action: Option<String>') &&
    has(src.adminAudit, 'target_type: Option<String>') &&
    has(src.adminAudit, 'clean_filter'),
  'audit list has safe actor/action/target_type filters'
);

check(
  has(src.adminAudit, 'value.chars().count() > 80') &&
    has(src.adminAudit, 'char::is_control'),
  'audit filters reject excessive/control input'
);

check(
  has(src.adminAudit, 'order by created_at desc, id desc'),
  'audit list is ordered newest-first'
);

check(
  has(src.adminAudit, 'where id = $1') &&
    has(src.adminAudit, 'Audit event was not found'),
  'audit detail fetches by event id and handles not found'
);

check(
  has(src.adminAudit, 'actor_kind') &&
    has(src.adminAudit, 'actor_user_id') &&
    has(src.adminAudit, 'action') &&
    has(src.adminAudit, 'target_type') &&
    has(src.adminAudit, 'target_user_id') &&
    has(src.adminAudit, 'target_id') &&
    has(src.adminAudit, 'capabilities') &&
    has(src.adminAudit, 'summary') &&
    has(src.adminAudit, 'metadata') &&
    has(src.adminAudit, 'created_at'),
  'audit event response includes required debug fields'
);

check(
  has(src.adminAuth, '"audit_read"') &&
    has(src.adminAuth, 'SUPER_ADMIN_CAPABILITIES') &&
    has(src.adminAuth, 'ADMIN_ALLOWED_CAPABILITIES'),
  'audit_read capability exists for superadmin/admin model'
);

check(
  has(src.adminAuth, 'pub async fn audit') &&
    has(src.adminAuth, 'insert into admin_audit_events') &&
    has(src.adminAuth, 'actor_kind, actor_user_id, action, target_type, target_user_id') &&
    has(src.adminAuth, 'target_id, capabilities, summary, metadata'),
  'shared admin audit writer inserts complete audit event'
);

check(
  has(src.adminSocial, 'admin_auth::audit') &&
    has(src.adminSocial, '"social_moderation_action"') &&
    has(src.adminSocial, '"capability"') &&
    has(src.adminSocial, '"report_id"'),
  'single social moderation action writes contextual audit event'
);

check(
  has(src.adminSocialBulk, 'admin_auth::audit') &&
    has(src.adminSocialBulk, '"social_bulk_moderation_job"') &&
    has(src.adminSocialBulk, '"social_bulk_moderation_item"') &&
    has(src.adminSocialBulk, '"dry_run"') &&
    has(src.adminSocialBulk, '"would_apply_count"') &&
    has(src.adminSocialBulk, '"applied_count"'),
  'bulk moderation writes job/item audit events with dry-run counters'
);

check(
  !hasAny(src.adminAudit, [
    'x-karyra-admin-token',
    'KARYRA_ADMIN_TOKEN'
  ]),
  'audit API does not echo admin token literals'
);

check(
  hasAny(all, [
    'tracing::error!',
    'database operation failed',
    'authorization failed'
  ]),
  'audit-related modules emit backend error logs',
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
  console.log('PASS 21A FAILED');
  process.exit(1);
}

console.log('');
console.log('PASS 21A OK');
