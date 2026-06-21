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

const reset = read('src/admin_reset.rs');
assertIncludes('reset neutral response', reset, 'If this email is eligible for admin recovery');
assertIncludes('reset review hierarchy superadmin', reset, '"superadmin" => true');
assertIncludes('reset review hierarchy admin moderator only', reset, '"admin" => target_role == Some("moderator")');
assertIncludes('reset artifact issue route', reset, '/requests/:request_id/recovery-artifacts');
assertIncludes('reset artifact hash', reset, 'let token_hash = hash_token(&token);');
assertIncludes('reset artifact bootstrap gate', reset, 'SPARK_ADMIN_RECOVERY_RETURN_BOOTSTRAP_TOKENS');
assertIncludes('reset artifact no mutation metadata', reset, '"credential_mutation": false');
assertNotIncludes('reset no direct credential execution', reset, 'new_password');
assertNotIncludes('reset no direct email change', reset, 'set email =');
assertNotIncludes('reset no direct totp revoke', reset, 'delete from admin_totp_factors');

const recovery = read('src/admin_recovery.rs');
assertIncludes('recovery inspect route', recovery, '.route("/inspect", post(inspect_recovery_artifact))');
assertIncludes('recovery password route', recovery, '.route("/password", post(execute_password_recovery))');
assertIncludes('recovery password type guard', recovery, 'artifact.request_type != "password"');
assertIncludes('recovery password hash', recovery, 'hash_password(&payload.new_password)');
assertIncludes('recovery password totp required', recovery, 'verify_totp_code(&secret, &payload.totp_code');
assertIncludes('recovery artifact consume', recovery, "set status = 'used'");
assertIncludes('recovery artifact used at', recovery, 'used_at = $2');
assertIncludes('recovery reset request completed', recovery, "set status = 'completed'");
assertIncludes('recovery completion metadata', recovery, '"completed_via": "admin_password_recovery"');
assertIncludes('recovery sessions revoked', recovery, 'update admin_sessions');
assertIncludes('recovery audit complete', recovery, 'admin_recovery_password_completed');
assertIncludes('recovery reset completed audit flag', recovery, '"reset_request_completed": true');
assertNotIncludes('recovery no email recovery execution yet', recovery, 'execute_email_recovery');
assertNotIncludes('recovery no totp recovery execution yet', recovery, 'execute_totp_recovery');
assertNotIncludes('recovery no email mutation yet', recovery, 'set email =');
assertNotIncludes('recovery no totp delete yet', recovery, 'delete from admin_totp_factors');

const migration = read('migrations/202606210001_admin_recovery_artifacts.sql');
assertIncludes('artifact migration table', migration, 'create table if not exists admin_recovery_artifacts');
assertIncludes('artifact migration token hash', migration, 'token_hash text not null unique');
assertIncludes('artifact migration status', migration, "status text not null default 'pending'");

const doc = read('docs/PASS_25E_Q_ADMIN_RECOVERY_CHAIN_AUDIT.md');
assertIncludes('runbook doc', doc, 'Password recovery marks the reset request `completed`.');
assertIncludes('runbook doc', doc, 'Email and 2FA recovery must not be exposed until separately implemented and audited.');
assertIncludes('runbook doc', doc, 'PASS 25E-R: 2FA recovery rotation design lock.');

console.log('PASS 25E-Q admin recovery chain audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: admin recovery chain is review-scoped, artifact-gated, password-only for execution, and completion-finalized.');
