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

const doc = read('docs/PASS_25E_T_ADMIN_EMAIL_RECOVERY_LOCK.md');
assertIncludes('email recovery doc', doc, 'no email recovery execution implemented in this pass');
assertIncludes('email recovery doc', doc, 'Approved `request_type = email` recovery artifact');
assertIncludes('email recovery doc', doc, 'OTP/code proof sent to the new email address');
assertIncludes('email recovery doc', doc, 'Old email and new email receive notifications');
assertIncludes('email recovery doc', doc, 'Final mutation is audited as `admin_recovery_email_completed`');

const recovery = read('src/admin_recovery.rs');
assertIncludes('recovery keeps password flow', recovery, '.route("/password", post(execute_password_recovery))');
assertIncludes('recovery keeps 2fa setup', recovery, '.route("/totp/setup", post(setup_totp_recovery))');
assertIncludes('recovery keeps 2fa confirm', recovery, '.route("/totp/confirm", post(confirm_totp_recovery))');
assertIncludes('email proof shell allowed', recovery, '.route("/email/request", post(request_email_recovery_otp))');
assertIncludes('email proof confirm allowed', recovery, '.route("/email/confirm", post(confirm_email_recovery_otp))');
assertIncludes('email artifact type guard', recovery, 'artifact.request_type != "email"');
assertIncludes('email proof no mutation flag', recovery, '"credential_mutation": false');
assertNotIncludes('no final email mutation', recovery, 'set email =');
assertNotIncludes('no final email completed audit yet', recovery, 'admin_recovery_email_completed');
assertNotIncludes('no direct change email marker', recovery, 'change_email');

const reset = read('src/admin_reset.rs');
assertIncludes('reset can request email recovery', reset, '"email"');
assertNotIncludes('review queue no email mutation', reset, 'set email =');
assertNotIncludes('review queue no final email mutation marker', reset, 'admin_recovery_email_completed');

console.log('PASS 25E-T backend admin email recovery lock audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: backend email recovery remains proof-only; final account email mutation is still locked.');
