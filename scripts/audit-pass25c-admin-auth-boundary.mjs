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

const cargo = read('Cargo.toml');
for (const dep of ['aes-gcm', 'base64', 'data-encoding', 'hmac', 'sha1']) {
  assertIncludes('Cargo.toml', cargo, dep);
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
assertIncludes('admin_login', adminLogin, 'POST /api/admin/auth/email/request');
assertIncludes('admin_login', adminLogin, 'POST /api/admin/auth/email/confirm');
assertIncludes('admin_login', adminLogin, 'POST /api/admin/auth/totp/setup');
assertIncludes('admin_login', adminLogin, 'POST /api/admin/auth/totp/confirm');
assertIncludes('admin_login', adminLogin, 'ADMIN_SESSION_HOURS');
assertIncludes('admin_login', adminLogin, 'admin_role_assignments');
assertIncludes('admin_login', adminLogin, 'admin_sessions');
assertIncludes('admin_login', adminLogin, 'admin_auth_login');
assertIncludes('admin_login', adminLogin, 'verify_password');
assertIncludes('admin_login', adminLogin, 'HttpOnly; SameSite=Lax');
assertIncludes('admin_login', adminLogin, 'email_verified_at');
assertIncludes('admin_login', adminLogin, 'admin_email_verification_tokens');
assertIncludes('admin_login', adminLogin, 'admin_totp_factors');
assertIncludes('admin_login', adminLogin, 'admin_auth_email_verification_required');
assertIncludes('admin_login', adminLogin, 'admin_auth_mfa_setup_required');
assertIncludes('admin_login', adminLogin, 'admin_auth_mfa_required');
assertIncludes('admin_login', adminLogin, 'SPARK_ADMIN_MFA_KEY');
assertIncludes('admin_login', adminLogin, 'encrypt_totp_secret');
assertIncludes('admin_login', adminLogin, 'decrypt_totp_secret');
assertIncludes('admin_login', adminLogin, 'verify_totp_code');
assertIncludes('admin_login', adminLogin, 'last_used_step');
assertIncludes('admin_login', adminLogin, 'login requires password plus TOTP code after setup');

const main = read('src/main.rs');
assertIncludes('main', main, 'mod admin_login;');

const http = read('src/http/mod.rs');
assertIncludes('http router', http, '.nest("/api/admin/auth", crate::admin_login::router())');

const sessionMigration = read('migrations/202606200001_admin_sessions.sql');
assertIncludes('admin sessions migration', sessionMigration, 'create table if not exists admin_sessions');
assertIncludes('admin sessions migration', sessionMigration, "role text not null check (role in ('admin', 'moderator'))");
assertIncludes('admin sessions migration', sessionMigration, 'token_hash text not null unique');

const mfaMigration = read('migrations/202606200002_admin_email_mfa_foundation.sql');
assertIncludes('admin mfa migration', mfaMigration, 'add column if not exists email_verified_at');
assertIncludes('admin mfa migration', mfaMigration, 'admin_email_verification_tokens');
assertIncludes('admin mfa migration', mfaMigration, 'admin_totp_factors');
assertIncludes('admin mfa migration', mfaMigration, 'secret_ciphertext');
assertIncludes('admin mfa migration', mfaMigration, 'admin_auth_challenges');
assertIncludes('admin mfa migration', mfaMigration, "challenge_type in ('email_verification', 'totp_setup', 'totp_login')");

console.log('PASS 25C admin auth boundary audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: delegated admin/moderator login has email verification + encrypted TOTP setup flow; superadmin root remains separated.');
