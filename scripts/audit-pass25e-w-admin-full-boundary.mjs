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

function assertAnyIncludes(label, content, needles) {
  if (!needles.some((needle) => content.includes(needle))) {
    failures.push(`${label}: missing one of ${needles.join(' | ')}`);
  }
}

function assertNotIncludes(label, content, needle) {
  if (content.includes(needle)) failures.push(`${label}: forbidden ${needle}`);
}

function assertFileHas(label, rel, checks) {
  const content = read(rel);
  for (const needle of checks) assertIncludes(label, content, needle);
  return content;
}

const main = read('src/main.rs');
assertIncludes('main recovery module', main, 'mod admin_recovery;');
assertIncludes('main reset module', main, 'mod admin_reset;');
assertIncludes('main onboarding module', main, 'mod admin_onboarding;');

const http = read('src/http/mod.rs');
assertIncludes('http recovery route', http, '.nest("/api/admin/recovery", crate::admin_recovery::router())');
assertIncludes('http reset route', http, '.nest("/api/admin/reset", crate::admin_reset::router())');
assertIncludes('http onboarding route', http, '.nest("/api/admin/onboarding", crate::admin_onboarding::router())');

const recovery = assertFileHas('admin recovery', 'src/admin_recovery.rs', [
  '.route("/inspect", post(inspect_recovery_artifact))',
  '.route("/password", post(execute_password_recovery))',
  '.route("/totp/setup", post(setup_totp_recovery))',
  '.route("/totp/confirm", post(confirm_totp_recovery))',
  '.route("/email/request", post(request_email_recovery_otp))',
  '.route("/email/confirm", post(confirm_email_recovery_otp))',
  '.route("/email/complete", post(complete_email_recovery))',
  'let token_hash = hash_token',
  "status = 'pending'",
  'expires_at > now()',
  'used_at is null',
  'revoked_at is null',
  'consume_artifact_and_complete_request_tx',
  'revoke_admin_sessions_tx'
]);
assertIncludes('password recovery guard', recovery, 'artifact.request_type != "password"');
assertIncludes('password recovery current totp', recovery, 'verify_totp_code(&secret, &payload.totp_code');
assertIncludes('password recovery audit', recovery, 'admin_recovery_password_completed');
assertIncludes('2fa recovery guard', recovery, 'artifact.request_type != "totp"');
assertIncludes('2fa recovery pending setup', recovery, 'pending_confirmation');
assertIncludes('2fa old factor delayed revoke', recovery, 'old_factor_revoked: false');
assertIncludes('2fa recovery audit', recovery, 'admin_recovery_totp_completed');
assertIncludes('email recovery guard', recovery, 'artifact.request_type != "email"');
assertIncludes('email proof hash check', recovery, "metadata->>'email_proof_token_hash'");
assertIncludes('email proof expiry check', recovery, "metadata->>'email_proof_expires_at'");
assertIncludes('email final mutation', recovery, 'set email = $2');
assertIncludes('email recovery audit', recovery, 'admin_recovery_email_completed');
assertIncludes('email notification pending honesty', recovery, 'notification_delivery_pending');
assertNotIncludes('no direct totp delete', recovery.toLowerCase(), 'delete from admin_totp_factors');
assertNotIncludes('no raw artifact manual token in recovery response', recovery, 'manual_token');

const reset = assertFileHas('admin reset', 'src/admin_reset.rs', [
  'admin_reset_requests',
  'admin_recovery_artifacts',
  'admin_reset_request_review',
  'admin_recovery_artifact_issue',
  'token_hash',
  'target_role',
  'can_review_target'
]);
assertIncludes('reset supports password type', reset, '"password"');
assertIncludes('reset supports email type', reset, '"email"');
assertIncludes('reset supports totp type', reset, '"totp"');
assertNotIncludes('reset review no direct password mutation', reset, 'set password_hash');
assertNotIncludes('reset review no direct email mutation', reset, 'set email =');
assertNotIncludes('reset review no direct totp delete', reset.toLowerCase(), 'delete from admin_totp_factors');

const onboarding = assertFileHas('admin onboarding', 'src/admin_onboarding.rs', [
  'admin_invitations',
  'admin_invite_email_otps',
  'hash_password',
  'encrypt_totp_secret',
  'verify_totp_code',
  'admin_invitation_accept'
]);
assertNotIncludes('onboarding final accept no duplicate fresh totp field', onboarding, 'totp_code: String');

const team = assertFileHas('admin team', 'src/admin_team.rs', [
  'admin_invitations',
  'token_hash',
  'admin_invitation_create'
]);
assertAnyIncludes('admin team role hierarchy', team, [
  'moderator',
  'admin'
]);

const auth = assertFileHas('admin auth', 'src/admin_auth.rs', [
  'superadmin',
  'admin',
  'moderator'
]);
assertAnyIncludes('auth capabilities sanitize', auth, ['sanitize_capabilities_for_role', 'canonical_role']);

assertFileHas('invite model migration', 'migrations/202606200003_admin_invite_only_model.sql', [
  'admin_invitations',
  'admin_invite_email_otps',
  'admin_reset_requests'
]);
assertFileHas('recovery artifact migration', 'migrations/202606210001_admin_recovery_artifacts.sql', [
  'admin_recovery_artifacts',
  'token_hash',
  "status text not null default 'pending'"
]);
assertFileHas('email proof migration', 'migrations/202606210002_admin_email_recovery_otps.sql', [
  'admin_email_recovery_otps',
  'otp_hash',
  'new_email'
]);

console.log('PASS 25E-W backend full admin boundary audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: backend admin invite, onboarding, reset, artifact, password, 2FA, and email recovery boundaries are intact.');
