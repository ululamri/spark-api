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

const outbox = read('src/admin_notification_outbox.rs');
assertIncludes('generic notification enqueue', outbox, 'enqueue_admin_notification');
assertIncludes('generic notification alias', outbox, 'pub type AdminNotification');
assertIncludes('shared outbox insert', outbox, 'admin_recovery_notification_outbox');
assertIncludes('pending insert', outbox, "'pending'");
assertNotIncludes('no dry run in outbox helper', outbox, 'dry-run');
assertNotIncludes('no dryrun in outbox helper', outbox, 'dryrun');
assertNotIncludes('no mock in outbox helper', outbox, 'mock');
assertNotIncludes('no simulated in outbox helper', outbox, 'simulated');

const team = read('src/admin_team.rs');
assertIncludes('invite email event', team, 'admin_invitation_created_email');
assertIncludes('invite onboarding link', team, '/admin/onboarding?token=');
assertIncludes('invite raw token queued only', team, 'onboarding_url_included');
assertIncludes('invite uses shared outbox', team, 'enqueue_admin_notification');
assertIncludes('invite recipient email', team, 'recipient_email: &email');

const onboarding = read('src/admin_onboarding.rs');
assertIncludes('otp email event', onboarding, 'admin_invite_email_otp_email');
assertIncludes('otp email body', onboarding, 'Your Karyra Spark admin onboarding code is');
assertIncludes('otp uses shared outbox', onboarding, 'enqueue_admin_notification');
assertIncludes('otp recipient email', onboarding, 'recipient_email: &email');

const mailer = read('src/admin_mailer.rs');
assertIncludes('smtp worker still reads pending outbox', mailer, "where status = 'pending'");
assertIncludes('smtp worker marks sent', mailer, "status = 'sent'");
assertIncludes('smtp worker marks failed', mailer, "status = 'failed'");

const doc = read('docs/PASS_25E_AA_ADMIN_INVITE_GMAIL_DELIVERY.md');
assertIncludes('doc invite event', doc, 'admin_invitation_created_email');
assertIncludes('doc otp event', doc, 'admin_invite_email_otp_email');
assertIncludes('doc no dry run', doc, 'There is no dry-run');

console.log('PASS 25E-AA admin invite Gmail delivery audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: admin invite and onboarding OTP emails are queued for real Gmail SMTP delivery without dry-run paths.');
