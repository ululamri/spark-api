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
  [files.http, '.nest("/api/admin/ai", crate::admin_ai::router())'],
  [files.http, '.nest("/api/admin/audit", crate::admin_audit::router())'],
  [files.http, '.nest("/api/admin/cms", crate::admin_cms::router())'],
  [files.http, '.nest("/api/admin/team", crate::admin_team::router())'],
  [files.http, '.nest("/api/admin/social/bulk", crate::admin_social_bulk::router())'],
  [files.http, '.nest("/api/admin/social/ml", crate::admin_social_ml::router())'],
  [files.http, '.nest("/api/admin/social/ops", crate::admin_social_ops::router())'],
  [files.http, '.nest("/api/admin/social", crate::admin_social::router())'],
  [files.http, '.nest("/api/admin", crate::admin::router())'],
  [files.auth, 'pub async fn authorize_admin_actor'],
  [files.auth, 'pub async fn authorize_with_capability'],
  [files.auth, 'pub fn authorize_super_admin_only'],
  [files.team, 'authorize_admin_actor'],
  [files.team, 'authorize_admin_manage'],
  [files.team, 'admin-rbac-final-matrix'],
  [files.audit, 'authorize_with_capability'],
  [files.social, 'authorize_with_capability'],
  [files.socialMl, 'authorize_with_capability'],
  [files.socialBulk, 'authorize_with_capability'],
  [files.socialOps, 'authorize_with_capability'],
  [files.ai, 'authorize_with_capability'],
  [files.cms, 'authorize_with_capability']
];

const warnings = [];
if (files.rootAdmin.includes('fn authorize(state: &AppState, headers: &HeaderMap)')) {
  warnings.push('src/admin/mod.rs still uses legacy root-token authorize() for root-only admin read endpoints. This is acceptable as a root-only bootstrap boundary, but it is not yet unified with admin_auth.rs.');
}
if (files.rootAdmin.includes('TODO(production): replace the bootstrap token with scoped RBAC')) {
  warnings.push('src/admin/mod.rs still has production TODO for scoped RBAC replacement.');
}

const blockers = required.filter(([text, needle]) => !text.includes(needle));
console.log('PASS 19E backend admin surface audit');
if (warnings.length) {
  console.log('\nWarnings:');
  for (const warning of warnings) console.log(`- ${warning}`);
}
if (blockers.length) {
  console.error('\nBlockers:');
  for (const [, needle] of blockers) console.error(`- Missing ${needle}`);
  process.exit(1);
}
console.log('\nNo PASS 19E backend admin surface blockers found.');
