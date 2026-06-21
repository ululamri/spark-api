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
assertIncludes('main module', main, 'mod admin_recovery;');

const http = read('src/http/mod.rs');
assertIncludes('http recovery router', http, '.nest("/api/admin/recovery", crate::admin_recovery::router())');

const recovery = read('src/admin_recovery.rs');
assertIncludes('recovery scope', recovery, 'admin-recovery-artifact-intake-shell');
assertIncludes('recovery route', recovery, '.route("/inspect", post(inspect_recovery_artifact))');
assertIncludes('recovery token hash', recovery, 'let token_hash = hash_token');
assertIncludes('recovery email match', recovery, 'lower(email) = lower($2)');
assertIncludes('recovery pending only', recovery, "status = 'pending'");
assertIncludes('recovery expiry guard', recovery, 'expires_at > now()');
assertIncludes('recovery used guard', recovery, 'used_at is null');
assertIncludes('recovery revoked guard', recovery, 'revoked_at is null');
assertIncludes('recovery mutation false', recovery, 'credential_mutation: false');
assertIncludes('recovery generic error', recovery, 'recovery artifact is invalid or expired');
assertNotIncludes('recovery no direct password mutation', recovery, 'password_hash =');
assertNotIncludes('recovery no direct email mutation', recovery, 'set email =');
assertNotIncludes('recovery no direct totp disable', recovery, 'enabled = false');
assertNotIncludes('recovery no artifact consumption', recovery, 'used_at = now()');
assertNotIncludes('recovery no raw token response', recovery, 'manual_token');

const doc = read('docs/PASS_25E_N_ADMIN_RECOVERY_INTAKE.md');
assertIncludes('recovery intake doc', doc, 'No credential mutation is implemented in this pass.');
assertIncludes('recovery intake doc', doc, 'Token is verified by hash only.');
assertIncludes('recovery intake doc', doc, 'Inspection does not update artifact state.');
assertIncludes('recovery intake doc', doc, '/admin/recovery');

console.log('PASS 25E-N backend admin recovery intake audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: backend recovery artifact intake verifies token/email without credential mutation or artifact consumption.');
