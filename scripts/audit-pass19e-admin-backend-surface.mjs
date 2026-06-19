#!/usr/bin/env node
import { readFileSync } from 'node:fs';

const files = {
  http: readFileSync('src/http/mod.rs', 'utf8'),
  rootAdmin: readFileSync('src/admin/mod.rs', 'utf8'),
  auth: readFileSync('src/admin_auth.rs', 'utf8'),
  team: readFileSync('src/admin_team.rs', 'utf8'),
  audit: readFileSync('src/admin_audit.rs', 'utf8'),
  social: readFileSync('src/admin_social.rs', 'utf8'),
  socialMl: readFileSync('src/admin_social_ml.rs', 'utf8'),
  socialBulk: readFileSync('src/admin_social_bulk.rs', 'utf8'),
  socialOps: readFileSync('src/admin_social_ops.rs', 'utf8'),
  ai: readFileSync('src/admin_ai.rs', 'utf8'),
  cms: readFileSync('src/admin_cms.rs', 'utf8')
};

const required = [
  ['src/http/mod.rs', files.http, '.nest("/api/admin/ai", crate::admin_ai::router())'],
  ['src/http/mod.rs', files.http, '.nest("/api/admin/audit", crate::admin_audit::router())'],
  ['src/http/mod.rs', files.http, '.nest("/api/admin/cms", crate::admin_cms::router())'],
  ['src/http/mod.rs', files.http, '.nest("/api/admin/team", crate::admin_team::router())'],
  ['src/http/mod.rs', files.http, '.nest("/api/admin/social/bulk", crate::admin_social_bulk::router())'],
  ['src/http/mod.rs', files.http, '.nest("/api/admin/social/ml", crate::admin_social_ml::router())'],
  ['src/http/mod.rs', files.http, '.nest("/api/admin/social/ops", crate::admin_social_ops::router())'],
  ['src/http/mod.rs', files.http, '.nest("/api/admin/social", crate::admin_social::router())'],
  ['src/http/mod.rs', files.http, '.nest("/api/admin", crate::admin::router())'],
  ['src/admin_auth.rs', files.auth, 'pub async fn authorize_admin_actor'],
  ['src/admin_auth.rs', files.auth, 'pub async fn authorize_with_capability'],
  ['src/admin_auth.rs', files.auth, 'pub fn authorize_super_admin_only'],
  ['src/admin_team.rs', files.team, 'authorize_admin_actor'],
  ['src/admin_team.rs', files.team, 'authorize_admin_manage'],
  ['src/admin_team.rs', files.team, 'admin-rbac-final-matrix'],
  ['src/admin_audit.rs', files.audit, 'authorize_with_capability'],
  ['src/admin_social.rs', files.social, 'authorize_with_capability'],
  ['src/admin_social_ml.rs', files.socialMl, 'authorize_with_capability'],
  ['src/admin_social_bulk.rs', files.socialBulk, 'authorize_with_capability'],
  ['src/admin_social_ops.rs', files.socialOps, 'authorize_with_capability'],
  ['src/admin_ai.rs', files.ai, 'authorize_with_capability'],
  ['src/admin_cms.rs', files.cms, 'authorize_with_capability'],
  ['src/admin/mod.rs', files.rootAdmin, 'admin_auth::authorize_super_admin_only(state, headers)']
];

const forbidden = [
  ['src/admin/mod.rs', files.rootAdmin, 'use sha2::{Digest, Sha256};'],
  ['src/admin/mod.rs', files.rootAdmin, 'const ADMIN_HEADER: &str = "x-karyra-admin-token";'],
  ['src/admin/mod.rs', files.rootAdmin, 'TODO(production): replace the bootstrap token with scoped RBAC'],
  ['src/admin_ai.rs', files.ai, 'use sha2::{Digest, Sha256};'],
  ['src/admin_ai.rs', files.ai, 'const ADMIN_HEADER: &str = "x-karyra-admin-token";']
];

const blockers = [];
for (const [path, text, needle] of required) {
  if (!text.includes(needle)) blockers.push(`${path}: missing ${needle}`);
}
for (const [path, text, needle] of forbidden) {
  if (text.includes(needle)) blockers.push(`${path}: forbidden legacy admin auth pattern ${needle}`);
}

console.log('PASS 19E backend admin surface audit');
if (blockers.length) {
  console.error('\nBlockers:');
  for (const blocker of blockers) console.error(`- ${blocker}`);
  process.exit(1);
}
console.log('\nNo PASS 19E backend admin surface blockers found.');
