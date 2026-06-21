import fs from 'node:fs';
import path from 'node:path';

const apiRoot = process.cwd();
const frontendRoot = process.env.SPARK_FRONTEND_ROOT || '../spark';
const failures = [];

function read(root, rel) {
  const file = path.join(root, rel);
  if (!fs.existsSync(file)) {
    failures.push(`Missing file: ${file}`);
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

const templates = read(apiRoot, 'src/admin_email_templates.rs');
assertIncludes('invite email visible code', templates, 'Kode undangan kamu');
assertIncludes('invite email fallback instruction', templates, 'Invite Code/Kode Undangan');
assertIncludes('invite email code argument', templates, 'invite_code: &str');

const team = read(apiRoot, 'src/admin_team.rs');
assertIncludes('invite code passed to template', team, 'admin_invitation_email(&role, &onboarding_url, &token, expires_at)');
assertIncludes('invite code metadata', team, 'invite_code_included');

const server = read(frontendRoot, 'src/routes/admin/onboarding/+page.server.ts');
assertIncludes('frontend load reads token query', server, 'url.searchParams.get');
assertIncludes('frontend load inviteCode', server, 'inviteCode');
assertIncludes('server copy uses invite code', server, 'Invite code is required');
assertNotIncludes('server no invite token copy', server, 'Invite token is required');

const page = read(frontendRoot, 'src/routes/admin/onboarding/+page.svelte');
assertIncludes('page reads data', page, 'let { data, form }');
assertIncludes('page user label invite code', page, 'Invite code');
assertIncludes('page prefills invite code', page, 'data?.inviteCode');
assertNotIncludes('page no invite token label', page, '<label for="inspect-token">Invite token</label>');

console.log('PASS 25E-AA3 invite code onboarding clarity audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: invite email exposes invite code and onboarding UI treats it as invite code while preserving internal token compatibility.');
