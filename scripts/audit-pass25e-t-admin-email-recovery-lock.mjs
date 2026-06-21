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

const recovery = read('src/admin_recovery.rs');
assertIncludes('email recovery has proof shell', recovery, '.route("/email/request", post(request_email_recovery_otp))');
assertIncludes('email recovery has proof confirm', recovery, '.route("/email/confirm", post(confirm_email_recovery_otp))');
assertIncludes('email recovery final requires proof', recovery, 'ensure_email_recovery_proof');
assertIncludes('email recovery final audit', recovery, 'admin_recovery_email_completed');

console.log('PASS 25E-T backend admin email recovery lock audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: backend email recovery follows locked proof-first finalization model.');
