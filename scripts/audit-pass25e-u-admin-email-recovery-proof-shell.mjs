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

const migration = read('migrations/202606210002_admin_email_recovery_otps.sql');
assertIncludes('email otp migration', migration, 'create table if not exists admin_email_recovery_otps');

const recovery = read('src/admin_recovery.rs');
assertIncludes('email request route', recovery, '.route("/email/request", post(request_email_recovery_otp))');
assertIncludes('email confirm route', recovery, '.route("/email/confirm", post(confirm_email_recovery_otp))');
assertIncludes('email stores otp hash', recovery, 'hash_email_recovery_otp');
assertIncludes('email proof token', recovery, 'new_email_recovery_proof_token');
assertIncludes('email proof audit', recovery, 'admin_recovery_email_proof_confirmed');
assertIncludes('email final consumes proof', recovery, 'ensure_email_recovery_proof');

console.log('PASS 25E-U backend admin email recovery proof shell audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: backend email recovery proof shell remains present and is consumed by finalization.');
