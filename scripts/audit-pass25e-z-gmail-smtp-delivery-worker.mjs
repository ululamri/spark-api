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

const cargo = read('Cargo.toml');
assertIncludes('lettre dependency', cargo, 'lettre =');
assertIncludes('lettre smtp transport', cargo, 'smtp-transport');
assertIncludes('lettre tokio runtime', cargo, 'tokio1');
assertIncludes('lettre rustls', cargo, 'tokio1-rustls');

const config = read('src/config.rs');
for (const key of [
  'mail_driver',
  'smtp_host',
  'smtp_port',
  'smtp_security',
  'smtp_username',
  'smtp_password',
  'mail_from',
  'mail_from_name'
]) assertIncludes(`mail config ${key}`, config, key);

const main = read('src/main.rs');
assertIncludes('mailer module', main, 'mod admin_mailer;');
assertIncludes('mailer spawn', main, 'spawn_smtp_delivery_worker');

const mailer = read('src/admin_mailer.rs');
assertIncludes('smtp worker', mailer, 'spawn_smtp_delivery_worker');
assertIncludes('smtp driver only', mailer, 'SPARK_MAIL_DRIVER=smtp');
assertIncludes('gmail starttls', mailer, 'starttls_relay');
assertIncludes('outbox pending query', mailer, "where status = 'pending'");
assertIncludes('sent status update', mailer, "status = 'sent'");
assertIncludes('failed status update', mailer, "status = 'failed'");
assertIncludes('failure reason', mailer, 'failure_reason');
assertNotIncludes('no dry run', mailer, 'dry-run');
assertNotIncludes('no dryrun', mailer, 'dryrun');
assertNotIncludes('no mock', mailer, 'mock');
assertNotIncludes('no simulated', mailer, 'simulated');
assertNotIncludes('no log only', mailer, 'log_only');

const doc = read('docs/PASS_25E_Z_GMAIL_SMTP_DELIVERY_WORKER.md');
assertIncludes('gmail doc', doc, 'smtp.gmail.com');
assertIncludes('gmail doc app password', doc, 'App Password');
assertIncludes('gmail doc no dry run', doc, 'There is no dry-run');

console.log('PASS 25E-Z Gmail SMTP delivery worker audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: real Gmail SMTP delivery worker is wired to admin recovery outbox with no dry-run path.');
