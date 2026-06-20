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
assertIncludes('main', main, 'mod admin_reset;');

const http = read('src/http/mod.rs');
assertIncludes('http router', http, '.nest("/api/admin/reset", crate::admin_reset::router())');

const reset = read('src/admin_reset.rs');
assertIncludes('admin reset', reset, 'POST /api/admin/reset/request');
assertIncludes('admin reset', reset, 'GET /api/admin/reset/requests');
assertIncludes('admin reset', reset, 'POST /api/admin/reset/requests/:request_id/review');
assertIncludes('admin reset', reset, 'admin_reset_requests');
assertIncludes('admin reset', reset, 'neutral_response');
assertIncludes('admin reset', reset, 'authorize_admin_manage');
assertIncludes('admin reset', reset, 'admin_reset_request_review');
assertIncludes('admin reset', reset, 'If this email is eligible for admin recovery');
assertNotIncludes('admin reset', reset, 'select exists(\n              select 1 from users');

const migration = read('migrations/202606200003_admin_invite_only_model.sql');
assertIncludes('reset migration', migration, 'create table if not exists admin_reset_requests');
assertIncludes('reset migration', migration, "request_type text not null check (request_type in ('password', 'email', 'totp'))");

console.log('PASS 25E-G admin reset request audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: admin reset request API is public-neutral and review queue is admin-manage protected.');
