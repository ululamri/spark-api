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
assertIncludes('recovery phase', recovery, 'admin-password-recovery-completion-finalization');
assertIncludes('recovery policy', recovery, 'password recovery marks the reset request completed');
assertIncludes('recovery response', recovery, 'reset_request_completed: bool');
assertIncludes('recovery completes reset request', recovery, "update admin_reset_requests");
assertIncludes('recovery sets completed', recovery, "set status = 'completed'");
assertIncludes('recovery only approved request', recovery, "and status = 'approved'");
assertIncludes('recovery completion metadata', recovery, 'completed_via');
assertIncludes('recovery audit completion', recovery, '"reset_request_completed": true');
assertIncludes('recovery response completion', recovery, 'reset_request_completed: true');
assertIncludes('recovery artifact used', recovery, "set status = 'used'");
assertIncludes('recovery sessions revoked', recovery, 'update admin_sessions');
assertNotIncludes('recovery no email mutation', recovery, 'set email =');
assertNotIncludes('recovery no totp disable', recovery, 'enabled = false');
assertNotIncludes('recovery no totp delete', recovery, 'delete from admin_totp_factors');

const doc = read('docs/PASS_25E_P_ADMIN_RECOVERY_COMPLETION_FINALIZATION.md');
assertIncludes('completion doc', doc, 'original reset request is marked `completed`');
assertIncludes('completion doc', doc, 'approved reset request could remain reusable');
assertIncludes('completion doc', doc, 'reset_request_completed: true');

console.log('PASS 25E-P admin recovery completion finalization audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: password recovery consumes artifact and completes the original reset request.');
