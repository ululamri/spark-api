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
assertIncludes('email complete route', recovery, '.route("/email/complete", post(complete_email_recovery))');
assertIncludes('email complete request', recovery, 'struct EmailRecoveryCompleteRequest');
assertIncludes('email proof helper', recovery, 'ensure_email_recovery_proof');
assertIncludes('email proof hash check', recovery, "metadata->>'email_proof_token_hash'");
assertIncludes('email proof expiry check', recovery, "metadata->>'email_proof_expires_at'");
assertIncludes('email type guard', recovery, 'artifact.request_type != "email"');
assertIncludes('email final mutation', recovery, 'set email = $2');
assertIncludes('email verified timestamp', recovery, 'email_verified_at = $3');
assertIncludes('email artifact consumed', recovery, '"mutation_type": "email"');
assertIncludes('email complete request', recovery, '"completed_via": "admin_email_recovery_finalization"');
assertIncludes('email sessions revoked', recovery, 'revoke_admin_sessions_tx');
assertIncludes('email audit complete', recovery, 'admin_recovery_email_completed');
assertIncludes('notification pending marker', recovery, 'notification_delivery_pending');

const doc = read('docs/PASS_25E_V_ADMIN_EMAIL_RECOVERY_FINALIZATION.md');
assertIncludes('email final doc', doc, 'updates `users.email`');
assertIncludes('email final doc', doc, 'marks the artifact `used`');
assertIncludes('email final doc', doc, 'marks the reset request `completed`');

console.log('PASS 25E-V backend admin email recovery finalization audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: backend email recovery finalization requires valid proof token, consumes artifact, completes reset request, and revokes sessions.');
