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
assertIncludes('2fa recovery requires artifact type', recovery, 'artifact.request_type != "totp"');
assertIncludes('2fa recovery requires password', recovery, 'verify_password(password_hash, &payload.password)');
assertIncludes('2fa recovery setup before rotation', recovery, 'pending_confirmation');
assertIncludes('2fa recovery old factor delayed revoke', recovery, 'old_factor_revoked: false');
assertIncludes('2fa recovery final rotation', recovery, 'admin_totp_recovery_rotation');
assertIncludes('2fa recovery completed audit', recovery, 'admin_recovery_totp_completed');
assertNotIncludes('no direct disable copy', recovery, 'disable_totp');
assertNotIncludes('no direct totp delete', recovery, 'delete from admin_totp_factors');
assertNotIncludes('no email recovery mutation', recovery, 'set email =');

console.log('PASS 25E-R admin 2FA recovery rotation lock audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: 2FA recovery remains rotation-based and avoids direct disable/delete patterns.');
