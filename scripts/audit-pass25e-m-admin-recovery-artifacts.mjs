import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const failures = [];

function read(rel) {
  const file = path.join(root, rel);
  if (!fs.existsSync(file)) {
    failures.push(`Missing file: ${rel}`);
    return '';
  }
  return fs.readFileSync(file, 'utf8');
}

function assertIncludes(label, content, needle) {
  if (!content.includes(needle)) failures.push(`${label}: missing ${needle}`);
}

function assertNotIncludes(label, content, needle) {
  if (content.includes(needle)) failures.push(`${label}: forbidden ${needle}`);
}

const migration = read('migrations/202606210001_admin_recovery_artifacts.sql');
assertIncludes('recovery artifacts migration', migration, 'create table if not exists admin_recovery_artifacts');
assertIncludes('recovery artifacts migration', migration, 'reset_request_id uuid not null references admin_reset_requests(id)');
assertIncludes('recovery artifacts migration', migration, 'token_hash text not null unique');
assertIncludes('recovery artifacts migration', migration, "request_type text not null check (request_type in ('password', 'email', 'totp'))");
assertIncludes('recovery artifacts migration', migration, "status text not null default 'pending'");
assertIncludes('recovery artifacts migration', migration, 'admin_recovery_artifacts_request_idx');

const reset = read('src/admin_reset.rs');
assertIncludes('admin reset route', reset, '.route("/requests/:request_id/recovery-artifacts", post(issue_recovery_artifact))');
assertIncludes('admin reset scope', reset, 'approved requests may issue a single-use short-lived recovery artifact');
assertIncludes('admin reset artifact minutes', reset, 'const RECOVERY_ARTIFACT_MINUTES: i64 = 45;');
assertIncludes('admin reset artifact struct', reset, 'struct RecoveryArtifactRow');
assertIncludes('admin reset artifact issue fn', reset, 'async fn issue_recovery_artifact');
assertIncludes('admin reset approved request guard', reset, 'fetch_approved_request');
assertIncludes('admin reset hierarchy guard', reset, 'can_review_target(&actor, request.target_role.as_deref())');
assertIncludes('admin reset duplicate guard', reset, 'an active recovery artifact already exists for this reset request');
assertIncludes('admin reset token hash', reset, 'let token_hash = hash_token(&token);');
assertIncludes('admin reset env gate', reset, 'SPARK_ADMIN_RECOVERY_RETURN_BOOTSTRAP_TOKENS');
assertIncludes('admin reset delivery default', reset, 'out_of_band_delivery_pending');
assertIncludes('admin reset audit', reset, 'admin_recovery_artifact_issue');
assertIncludes('admin reset no credential mutation metadata', reset, '"credential_mutation": false');
assertNotIncludes('admin reset no direct password mutation', reset, 'password_hash =');
assertNotIncludes('admin reset no direct email mutation', reset, 'set email =');
assertNotIncludes('admin reset no direct totp disable', reset, 'enabled = false');
assertNotIncludes('admin reset no direct totp delete', reset, 'delete from admin_mfa_factors');

const doc = read('docs/PASS_25E_M_ADMIN_RECOVERY_ARTIFACTS.md');
assertIncludes('recovery artifacts doc', doc, 'No credential mutation is implemented in this pass.');
assertIncludes('recovery artifacts doc', doc, 'Token is stored as `token_hash` only.');
assertIncludes('recovery artifacts doc', doc, 'SPARK_ADMIN_RECOVERY_RETURN_BOOTSTRAP_TOKENS=true');
assertIncludes('recovery artifacts doc', doc, 'Artifact expires after 45 minutes.');
assertIncludes('recovery artifacts doc', doc, 'Admin may issue artifacts only for approved moderator requests.');

console.log('PASS 25E-M admin recovery artifact audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: approved reset requests can issue hashed short-lived recovery artifacts without credential mutation.');
