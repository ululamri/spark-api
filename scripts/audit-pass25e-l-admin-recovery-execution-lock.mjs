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

const doc = read('docs/PASS_25E_L_ADMIN_RECOVERY_EXECUTION_LOCK.md');
assertIncludes('recovery execution lock doc', doc, 'Reset request review is only an approval record.');
assertIncludes('recovery execution lock doc', doc, 'Direct password change button on the review page.');
assertIncludes('recovery execution lock doc', doc, 'Direct email replacement button on the review page.');
assertIncludes('recovery execution lock doc', doc, 'Direct 2FA disable button on the review page.');
assertIncludes('recovery execution lock doc', doc, 'Single-use, short-lived recovery artifact issuance after approval.');
assertIncludes('recovery execution lock doc', doc, 'Recovery artifact consumption.');
assertIncludes('recovery execution lock doc', doc, 'Multi-superadmin database-backed root authority.');

const reset = read('src/admin_reset.rs');
assertIncludes('admin reset review only', reset, 'approval records review only; credential reset remains a separate recovery flow');
assertIncludes('admin reset neutral response', reset, 'If this email is eligible for admin recovery');
assertIncludes('admin reset hierarchy', reset, '"admin" => target_role == Some("moderator")');
assertIncludes('admin reset artifact issue route', reset, 'issue_recovery_artifact');
assertIncludes('admin reset artifact metadata', reset, '"credential_mutation": false');
assertNotIncludes('admin reset no direct password mutation', reset, 'password_hash =');
assertNotIncludes('admin reset no direct email mutation', reset, 'set email =');
assertNotIncludes('admin reset no direct totp disable', reset, 'enabled = false');
assertNotIncludes('admin reset no direct totp delete', reset, 'delete from admin_mfa_factors');
assertNotIncludes('admin reset no raw recovery token naming', reset, 'recovery_token');

const http = read('src/http/mod.rs');
assertIncludes('admin reset router mounted', http, '.nest("/api/admin/reset", crate::admin_reset::router())');
assertNotIncludes('no recovery execution router', http, 'admin_recovery');

console.log('PASS 25E-L admin recovery execution lock audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: recovery approval/artifact issuance remains separated from credential mutation endpoints.');
