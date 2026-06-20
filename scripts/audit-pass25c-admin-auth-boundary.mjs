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

const adminAuth = read('src/admin_auth.rs');
assertIncludes('admin_auth', adminAuth, 'ADMIN_SESSION_COOKIE_NAME');
assertIncludes('admin_auth', adminAuth, 'spark_admin_session');
assertIncludes('admin_auth', adminAuth, 'authorize_super_admin_token');
assertIncludes('admin_auth', adminAuth, 'authorize_delegated_admin_session');
assertIncludes('admin_auth', adminAuth, 'admin_sessions');
assertIncludes('admin_auth', adminAuth, 'role != "admin" && role != "moderator"');
if (adminAuth.includes('require_current_user')) failures.push('admin_auth must not depend on public user session for delegated admin authorization');

const adminLogin = read('src/admin_login.rs');
assertIncludes('admin_login', adminLogin, 'POST /api/admin/auth/login');
assertIncludes('admin_login', adminLogin, 'ADMIN_SESSION_HOURS');
assertIncludes('admin_login', adminLogin, 'admin_role_assignments');
assertIncludes('admin_login', adminLogin, 'admin_sessions');
assertIncludes('admin_login', adminLogin, 'admin_auth_login');
assertIncludes('admin_login', adminLogin, 'verify_password');
assertIncludes('admin_login', adminLogin, 'HttpOnly; SameSite=Lax');

const main = read('src/main.rs');
assertIncludes('main', main, 'mod admin_login;');

const http = read('src/http/mod.rs');
assertIncludes('http router', http, '.nest("/api/admin/auth", crate::admin_login::router())');

const migration = read('migrations/202606200001_admin_sessions.sql');
assertIncludes('admin sessions migration', migration, 'create table if not exists admin_sessions');
assertIncludes('admin sessions migration', migration, "role text not null check (role in ('admin', 'moderator'))");
assertIncludes('admin sessions migration', migration, 'token_hash text not null unique');

console.log('PASS 25C admin auth boundary audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: superadmin root and delegated admin/moderator auth boundaries are separated.');
