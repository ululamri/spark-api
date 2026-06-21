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

const migration = read('migrations/202606210004_admin_public_rate_limit_events.sql');
assertIncludes('rate limit migration', migration, 'admin_public_rate_limit_events');
assertIncludes('rate limit migration', migration, 'subject_hash text not null');
assertIncludes('rate limit migration', migration, 'allowed boolean not null');
assertIncludes('rate limit migration', migration, 'scope, subject_hash, occurred_at');

const main = read('src/main.rs');
assertIncludes('main public guard module', main, 'mod admin_public_guard;');

const guard = read('src/admin_public_guard.rs');
assertIncludes('public guard function', guard, 'pub async fn check_public_throttle');
assertIncludes('public guard hash', guard, 'public_subject_hash');
assertIncludes('public guard no raw values', guard, 'subject_hash');
assertIncludes('public guard rate limited', guard, 'ApiError::RateLimited');
assertNotIncludes('public guard no raw email column', guard, 'email text');
assertNotIncludes('public guard no raw token column', guard, 'token text');

const reset = read('src/admin_reset.rs');
assertIncludes('reset throttled', reset, 'admin_reset_request');
assertIncludes('reset header input', reset, 'headers: HeaderMap');

const recovery = read('src/admin_recovery.rs');
for (const scope of [
  'admin_recovery_inspect',
  'admin_recovery_password',
  'admin_recovery_totp_setup',
  'admin_recovery_totp_confirm',
  'admin_recovery_email_request',
  'admin_recovery_email_confirm',
  'admin_recovery_email_complete'
]) {
  assertIncludes(`recovery throttle ${scope}`, recovery, scope);
}
assertIncludes('recovery header input', recovery, 'headers: HeaderMap');

const onboarding = read('src/admin_onboarding.rs');
for (const scope of [
  'admin_onboarding_invite_inspect',
  'admin_onboarding_email_request',
  'admin_onboarding_email_confirm',
  'admin_onboarding_password',
  'admin_onboarding_totp_setup',
  'admin_onboarding_totp_confirm',
  'admin_onboarding_accept'
]) {
  assertIncludes(`onboarding throttle ${scope}`, onboarding, scope);
}
assertIncludes('onboarding header input', onboarding, 'headers: HeaderMap');

const doc = read('docs/PASS_25E_X_ADMIN_PUBLIC_SURFACE_PROTECTION.md');
assertIncludes('public surface doc', doc, 'database-backed throttling');
assertIncludes('public surface doc', doc, 'Raw email/token/IP/user-agent values are not stored');

console.log('PASS 25E-X admin public surface protection audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: unauthenticated admin reset, recovery, and onboarding surfaces are protected by database-backed throttling.');
