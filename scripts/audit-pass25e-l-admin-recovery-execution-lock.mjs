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

const reset = read('src/admin_reset.rs');
assertIncludes('admin reset review only', reset, 'approval records review only; credential reset remains a separate recovery flow');
assertIncludes('admin reset artifact issue route', reset, 'issue_recovery_artifact');
assertNotIncludes('admin reset no direct password mutation', reset, 'password_hash =');
assertNotIncludes('admin reset no direct email mutation', reset, 'set email =');
assertNotIncludes('admin reset no direct totp disable', reset, 'enabled = false');
assertNotIncludes('admin reset no direct totp delete', reset, 'delete from admin_mfa_factors');

const recovery = read('src/admin_recovery.rs');
assertIncludes('admin recovery inspect route', recovery, '.route("/inspect", post(inspect_recovery_artifact))');
assertIncludes('admin recovery password route', recovery, '.route("/password", post(execute_password_recovery))');
assertIncludes('admin recovery password type guard', recovery, 'artifact.request_type != "password"');
assertIncludes('admin recovery requires totp', recovery, 'verify_totp_code(&secret, &payload.totp_code');
assertIncludes('admin recovery consumes artifact', recovery, "set status = 'used'");
assertIncludes('admin recovery revokes sessions', recovery, 'update admin_sessions');
assertNotIncludes('admin recovery no direct email mutation', recovery, 'set email =');
assertNotIncludes('admin recovery no direct totp disable', recovery, 'enabled = false');
assertNotIncludes('admin recovery no direct totp delete', recovery, 'delete from admin_totp_factors');

const http = read('src/http/mod.rs');
assertIncludes('admin reset router mounted', http, '.nest("/api/admin/reset", crate::admin_reset::router())');
assertIncludes('admin recovery router mounted', http, '.nest("/api/admin/recovery", crate::admin_recovery::router())');

console.log('PASS 25E-L admin recovery execution lock audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: reset approval/artifact issue remain separated; password recovery is the only enabled credential mutation and requires TOTP.');
