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

const migration = read('migrations/202606210002_admin_email_recovery_otps.sql');
assertIncludes('email otp migration', migration, 'create table if not exists admin_email_recovery_otps');
assertIncludes('email otp migration', migration, 'artifact_id uuid not null references admin_recovery_artifacts');
assertIncludes('email otp migration', migration, 'new_email text not null');
assertIncludes('email otp migration', migration, 'otp_hash text not null');

const recovery = read('src/admin_recovery.rs');
assertIncludes('email request route', recovery, '.route("/email/request", post(request_email_recovery_otp))');
assertIncludes('email confirm route', recovery, '.route("/email/confirm", post(confirm_email_recovery_otp))');
assertIncludes('email type guard', recovery, 'artifact.request_type != "email"');
assertIncludes('email requires password', recovery, 'verify_password(&password_hash, &payload.password)');
assertIncludes('email requires totp', recovery, 'verify_totp_code(&secret, &payload.totp_code');
assertIncludes('email validates availability', recovery, 'ensure_email_available');
assertIncludes('email stores otp hash', recovery, 'hash_email_recovery_otp');
assertIncludes('email proof token', recovery, 'new_email_recovery_proof_token');
assertIncludes('email bootstrap gate', recovery, 'SPARK_ADMIN_EMAIL_RECOVERY_RETURN_BOOTSTRAP_TOKENS');
assertIncludes('email proof audit', recovery, 'admin_recovery_email_proof_confirmed');
assertNotIncludes('no final email mutation yet', recovery, 'set email =');
assertNotIncludes('no final email audit yet', recovery, 'admin_recovery_email_completed');
assertNotIncludes('no change email marker', recovery, 'change_email');

const doc = read('docs/PASS_25E_U_ADMIN_EMAIL_RECOVERY_PROOF_SHELL.md');
assertIncludes('email proof doc', doc, 'No account email mutation is implemented in this pass.');
assertIncludes('email proof doc', doc, 'Recovery artifact remains pending.');
assertIncludes('email proof doc', doc, 'PASS 25E-V should implement final email mutation');

console.log('PASS 25E-U backend admin email recovery proof shell audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: backend email recovery proof shell creates OTP proof without mutating account email.');
