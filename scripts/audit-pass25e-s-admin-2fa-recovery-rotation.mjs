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
assertIncludes('2fa setup route', recovery, '.route("/totp/setup", post(setup_totp_recovery))');
assertIncludes('2fa confirm route', recovery, '.route("/totp/confirm", post(confirm_totp_recovery))');
assertIncludes('2fa request type guard', recovery, 'artifact.request_type != "totp"');
assertIncludes('2fa requires password', recovery, 'verify_password(password_hash, &payload.password)');
assertIncludes('2fa creates pending factor', recovery, 'Karyra Spark Admin Recovery');
assertIncludes('2fa pending metadata', recovery, '"status": "pending_confirmation"');
assertIncludes('2fa old not revoked at setup', recovery, 'old_factor_revoked: false');
assertIncludes('2fa confirm pending factor', recovery, 'load_pending_recovery_totp_factor');
assertIncludes('2fa confirm code', recovery, 'verify_totp_code(&secret, &payload.code');
assertIncludes('2fa revoke old on confirm', recovery, 'revoked_via');
assertIncludes('2fa enable new factor', recovery, 'enabled_via');
assertIncludes('2fa consume artifact', recovery, "set status = 'used'");
assertIncludes('2fa complete request', recovery, "set status = 'completed'");
assertIncludes('2fa audit', recovery, 'admin_recovery_totp_completed');
assertIncludes('2fa sessions revoked', recovery, 'revoke_admin_sessions_tx');
assertNotIncludes('no email recovery mutation', recovery, 'set email =');
assertNotIncludes('no direct totp delete', recovery, 'delete from admin_totp_factors');

const doc = read('docs/PASS_25E_S_ADMIN_2FA_RECOVERY_ROTATION_FLOW.md');
assertIncludes('2fa doc', doc, 'No direct 2FA disable endpoint.');
assertIncludes('2fa doc', doc, 'Existing TOTP is revoked only after the replacement factor is verified.');
assertIncludes('2fa doc', doc, 'Email recovery remains unimplemented.');

console.log('PASS 25E-S backend admin 2FA recovery rotation audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: backend 2FA recovery rotates TOTP only after new factor confirmation.');
