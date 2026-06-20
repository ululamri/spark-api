import fs from 'node:fs';

const src = fs.readFileSync('src/admin_onboarding.rs', 'utf8');
const failures = [];
const acceptStruct = src.match(/struct InviteAcceptRequest \{[\s\S]*?\n\}/)?.[0] ?? '';
const acceptFn = src.match(/async fn accept_invite\([\s\S]*?\n\}/)?.[0] ?? '';

if (acceptStruct.includes('totp_code')) failures.push('InviteAcceptRequest must not require totp_code.');
if (acceptFn.includes('payload.totp_code')) failures.push('accept_invite must not verify a second TOTP code.');
if (acceptFn.includes('verify_totp_code(&secret')) failures.push('accept_invite must not call verify_totp_code.');
if (!acceptFn.includes('admin 2FA must be enabled before accepting invite')) failures.push('accept_invite must still require enabled 2FA before activation.');
if (!acceptFn.includes('select id\n        from admin_totp_factors')) failures.push('accept_invite must check enabled TOTP factor by id.');

console.log('PASS 25E-F backend onboarding no-double-TOTP audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: accept invite requires enabled 2FA but no longer asks for a second TOTP code.');
