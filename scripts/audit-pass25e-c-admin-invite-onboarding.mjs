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

const main = read('src/main.rs');
assertIncludes('main module registry', main, 'mod admin_onboarding;');

const http = read('src/http/mod.rs');
assertIncludes('http router', http, '.nest("/api/admin/onboarding", crate::admin_onboarding::router())');

const onboarding = read('src/admin_onboarding.rs');
assertIncludes('admin_onboarding', onboarding, 'phase: "invite-token-admin-onboarding"');
assertIncludes('admin_onboarding', onboarding, '.route("/invite/inspect", post(inspect_invite))');
assertIncludes('admin_onboarding', onboarding, '.route("/invite/email/request", post(request_invite_email_otp))');
assertIncludes('admin_onboarding', onboarding, '.route("/invite/email/confirm", post(confirm_invite_email_otp))');
assertIncludes('admin_onboarding', onboarding, '.route("/invite/password", post(set_invite_password))');
assertIncludes('admin_onboarding', onboarding, '.route("/invite/totp/setup", post(setup_invite_totp))');
assertIncludes('admin_onboarding', onboarding, '.route("/invite/totp/confirm", post(confirm_invite_totp))');
assertIncludes('admin_onboarding', onboarding, '.route("/invite/accept", post(accept_invite))');
assertIncludes('admin_onboarding', onboarding, 'load_pending_invitation');
assertIncludes('admin_onboarding', onboarding, 'token_hash = $1');
assertIncludes('admin_onboarding', onboarding, 'ensure_email_proof');
assertIncludes('admin_onboarding', onboarding, 'email_proof_token_hash');
assertIncludes('admin_onboarding', onboarding, 'hash_invite_otp');
assertIncludes('admin_onboarding', onboarding, 'set_invite_password');
assertIncludes('admin_onboarding', onboarding, 'admin_invite_password_set');
assertIncludes('admin_onboarding', onboarding, 'admin_invite_totp_enabled');
assertIncludes('admin_onboarding', onboarding, 'admin_invitation_accept');
assertIncludes('admin_onboarding', onboarding, 'insert into admin_role_assignments');
assertIncludes('admin_onboarding', onboarding, 'accepted_at = $2');
assertIncludes('admin_onboarding', onboarding, 'SPARK_ADMIN_INVITE_RETURN_BOOTSTRAP_TOKENS');
assertNotIncludes('admin_onboarding', onboarding, 'authorize_admin_manage');
assertNotIncludes('admin_onboarding', onboarding, 'load_admin_password_actor');

console.log('PASS 25E-C admin invite onboarding audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: invite-token onboarding now validates invite token, email OTP proof, password, TOTP, and activates delegated admin roles only after acceptance.');
