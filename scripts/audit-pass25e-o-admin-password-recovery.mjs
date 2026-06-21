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

const recovery = read('src/admin_recovery.rs');
assertIncludes('recovery password route', recovery, '.route("/password", post(execute_password_recovery))');
assertIncludes('recovery password request', recovery, 'struct PasswordRecoveryRequest');
assertIncludes('recovery request type guard', recovery, 'artifact.request_type != "password"');
assertIncludes('recovery password validation', recovery, 'validate_password(&payload.new_password)');
assertIncludes('recovery password hash', recovery, 'hash_password(&payload.new_password)');
assertIncludes('recovery requires totp', recovery, 'verify_totp_code(&secret, &payload.totp_code');
assertIncludes('recovery consumes artifact', recovery, "set status = 'used'");
assertIncludes('recovery marks used_at', recovery, 'used_at = $2');
assertIncludes('recovery revokes sessions', recovery, 'update admin_sessions');
assertIncludes('recovery audit', recovery, 'admin_recovery_password_completed');
assertIncludes('recovery mutation flag', recovery, '"credential_mutation": true');
assertNotIncludes('recovery no email change', recovery, 'set email =');
assertNotIncludes('recovery no totp disable', recovery, 'enabled = false');
assertNotIncludes('recovery no totp delete', recovery, 'delete from admin_totp_factors');
assertNotIncludes('recovery no session creation', recovery, 'insert into admin_sessions');

const doc = read('docs/PASS_25E_O_ADMIN_PASSWORD_RECOVERY.md');
assertIncludes('password recovery doc', doc, 'implemented for password recovery only');
assertIncludes('password recovery doc', doc, 'current TOTP code');
assertIncludes('password recovery doc', doc, 'artifact is marked `used`');
assertIncludes('password recovery doc', doc, 'active delegated admin sessions for the user are revoked');
assertIncludes('password recovery doc', doc, 'does not implement email recovery or 2FA recovery execution');

console.log('PASS 25E-O backend admin password recovery audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: password recovery consumes approved artifact with TOTP and no email/2FA mutation.');
