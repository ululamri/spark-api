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

const doc = read('docs/PASS_25E_R_ADMIN_2FA_RECOVERY_ROTATION_LOCK.md');
assertIncludes('2fa recovery lock doc', doc, 'No 2FA credential mutation is implemented in this pass.');
assertIncludes('2fa recovery lock doc', doc, '2FA recovery must be a rotation flow');
assertIncludes('2fa recovery lock doc', doc, 'Only after new factor confirmation');
assertIncludes('2fa recovery lock doc', doc, 'POST /api/admin/recovery/totp/setup');
assertIncludes('2fa recovery lock doc', doc, 'POST /api/admin/recovery/totp/confirm');
assertIncludes('2fa recovery lock doc', doc, 'No old factor is revoked until replacement is confirmed.');

const recovery = read('src/admin_recovery.rs');
assertIncludes('password recovery remains active', recovery, '.route("/password", post(execute_password_recovery))');
assertIncludes('password recovery still consumes artifact', recovery, "set status = 'used'");
assertIncludes('password recovery still completes request', recovery, "set status = 'completed'");
assertNotIncludes('no totp recovery setup route yet', recovery, '/totp/setup');
assertNotIncludes('no totp recovery confirm route yet', recovery, '/totp/confirm');
assertNotIncludes('no totp disable route', recovery, '/totp/disable');
assertNotIncludes('no totp revoke route', recovery, '/totp/revoke');
assertNotIncludes('no direct totp disable', recovery, 'enabled_at = null');
assertNotIncludes('no direct totp revoke before replacement flow marker', recovery, 'revoke_old_totp_factors');
assertNotIncludes('no delete totp factors', recovery, 'delete from admin_totp_factors');

const reset = read('src/admin_reset.rs');
assertIncludes('reset supports totp request type', reset, '"password" | "email" | "totp"');
assertIncludes('reset review hierarchy preserved', reset, '"admin" => target_role == Some("moderator")');
assertNotIncludes('review page backend cannot disable totp', reset, 'disable_totp');
assertNotIncludes('review page backend cannot revoke totp', reset, 'revoke_totp');
assertNotIncludes('review page backend cannot delete totp', reset, 'delete from admin_totp_factors');

console.log('PASS 25E-R backend admin 2FA recovery rotation lock audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: 2FA recovery remains locked to future rotation design; no direct disable/revoke recovery exists.');
