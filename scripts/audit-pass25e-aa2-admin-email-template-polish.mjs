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
  if (content.toLowerCase().includes(needle.toLowerCase())) failures.push(`${label}: forbidden ${needle}`);
}

const main = read('src/main.rs');
assertIncludes('template module wired', main, 'mod admin_email_templates;');

const templates = read('src/admin_email_templates.rs');
for (const needle of [
  'Karyra Spark Admin Panel',
  'Halo Sahabat Karyra',
  'Tim Karyra Spark',
  'Email ini dikirim otomatis',
  'admin_invitation_email',
  'admin_invite_email_otp',
  'password_recovery_completed_email',
  'totp_recovery_completed_email',
  'email_recovery_old_address_notice',
  'email_recovery_new_address_notice'
]) assertIncludes('template content', templates, needle);

const team = read('src/admin_team.rs');
assertIncludes('invite template used', team, 'admin_invitation_email');
assertIncludes('invite subject polished', team, 'Undangan Karyra Spark Admin Panel');
assertIncludes('invite queued to outbox', team, 'admin_invitation_created_email');

const onboarding = read('src/admin_onboarding.rs');
assertIncludes('otp template used', onboarding, 'admin_invite_email_otp');
assertIncludes('otp subject polished', onboarding, 'Kode onboarding Karyra Spark Admin Panel');
assertIncludes('otp queued to outbox', onboarding, 'admin_invite_email_otp_email');

const recovery = read('src/admin_recovery.rs');
assertIncludes('password recovery template used', recovery, 'password_recovery_completed_email');
assertIncludes('totp recovery template used', recovery, 'totp_recovery_completed_email');
assertIncludes('email old recovery template used', recovery, 'email_recovery_old_address_notice');
assertIncludes('email new recovery template used', recovery, 'email_recovery_new_address_notice');

const all = [templates, team, onboarding, recovery].join('\n');
for (const forbidden of ['dry-run', 'dryrun', 'mock', 'simulated', 'fake sent']) {
  assertNotIncludes('no fake delivery wording', all, forbidden);
}

console.log('PASS 25E-AA2 admin email template polish audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: admin auth emails use polished branded templates while keeping real SMTP delivery only.');
