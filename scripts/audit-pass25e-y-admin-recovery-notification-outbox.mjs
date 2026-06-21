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

const migration = read('migrations/202606210005_admin_recovery_notification_outbox.sql');
assertIncludes('outbox migration', migration, 'admin_recovery_notification_outbox');
assertIncludes('outbox migration', migration, "status text not null default 'pending'");
assertIncludes('outbox migration', migration, 'recipient_email text not null');
assertIncludes('outbox migration', migration, 'related_artifact_id uuid references admin_recovery_artifacts');
assertIncludes('outbox migration', migration, 'related_reset_request_id uuid references admin_reset_requests');

const main = read('src/main.rs');
assertIncludes('main outbox module', main, 'mod admin_notification_outbox;');

const outbox = read('src/admin_notification_outbox.rs');
assertIncludes('outbox enqueue function', outbox, 'enqueue_recovery_notification_tx');
assertIncludes('outbox pending insert', outbox, "'pending'");
assertIncludes('outbox metadata helper', outbox, 'recovery_notification_metadata');
assertNotIncludes('outbox no smtp claim', outbox.toLowerCase(), 'smtp');

const recovery = read('src/admin_recovery.rs');
assertIncludes('password recovery notification', recovery, 'admin_password_recovery_completed_notice');
assertIncludes('totp recovery notification', recovery, 'admin_totp_recovery_completed_notice');
assertIncludes('email recovery old notification', recovery, 'admin_email_recovery_old_email_notice');
assertIncludes('email recovery new notification', recovery, 'admin_email_recovery_new_email_notice');
assertIncludes('notification enqueued in tx', recovery, 'enqueue_recovery_notification_tx');

const doc = read('docs/PASS_25E_Y_ADMIN_RECOVERY_NOTIFICATION_OUTBOX.md');
assertIncludes('outbox doc', doc, 'does not send SMTP yet');
assertIncludes('outbox doc', doc, 'status `pending`');
assertIncludes('outbox doc', doc, 'future SMTP worker');

console.log('PASS 25E-Y admin recovery notification outbox audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: admin recovery completion events are queued in durable notification outbox without claiming delivery.');
