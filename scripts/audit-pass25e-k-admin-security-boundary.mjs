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

function blockAfter(content, startNeedle, endNeedle) {
  const start = content.indexOf(startNeedle);
  if (start < 0) return '';
  const end = content.indexOf(endNeedle, start + startNeedle.length);
  if (end < 0) return content.slice(start);
  return content.slice(start, end + endNeedle.length);
}

const auth = read('src/admin_auth.rs');
const adminCaps = blockAfter(auth, 'pub const ADMIN_ALLOWED_CAPABILITIES', '];');
const moderatorCaps = blockAfter(auth, 'pub const MODERATOR_ALLOWED_CAPABILITIES', '];');
assertIncludes('admin auth', auth, 'pub const SUPER_ADMIN_CAPABILITIES');
assertIncludes('admin auth superadmin caps', auth, '"admin_manage"');
assertIncludes('admin auth', auth, 'pub fn canonical_role');
assertIncludes('admin auth', auth, '"sub_admin" => "admin".to_string()');
assertNotIncludes('delegated admin caps', adminCaps, '"admin_manage"');
assertNotIncludes('delegated moderator caps', moderatorCaps, '"admin_manage"');
assertNotIncludes('delegated moderator caps', moderatorCaps, '"audit_read"');

const team = read('src/admin_team.rs');
assertIncludes('admin team scope', team, 'superadmin can invite admin or moderator');
assertIncludes('admin team scope', team, 'admin can invite moderator only');
assertIncludes('admin team scope', team, 'moderator cannot invite');
assertIncludes('admin team invite policy', team, '("superadmin", "admin" | "moderator") => Ok(())');
assertIncludes('admin team invite policy', team, '("admin", "moderator") => Ok(())');
assertIncludes('admin team invite policy', team, '("admin", "admin") => Err(ApiError::Unauthorized)');
assertIncludes('admin team invite policy', team, '("moderator", _) => Err(ApiError::Unauthorized)');
assertIncludes('admin team invitation scope', team, "or (role = 'moderator' and invited_by_user_id = $4)");
assertIncludes('admin team token hash', team, 'let token_hash = hash_token(&token);');
assertIncludes('admin team token hash', team, 'manual_token: if return_token { Some(token) } else { None }');
assertNotIncludes('admin team direct grant', team, '.route("/members", get(members).post');

const onboarding = read('src/admin_onboarding.rs');
assertIncludes('admin onboarding routes', onboarding, '.route("/invite/email/request", post(request_invite_email_otp))');
assertIncludes('admin onboarding routes', onboarding, '.route("/invite/totp/confirm", post(confirm_invite_totp))');
assertIncludes('admin onboarding gates', onboarding, 'raw invite token is never stored');
assertIncludes('admin onboarding gates', onboarding, 'OTP is hashed and single-use');
assertIncludes('admin onboarding gates', onboarding, 'role assignment is created only after password plus enabled TOTP');
const acceptRequest = blockAfter(onboarding, 'struct InviteAcceptRequest', '}');
assertIncludes('admin onboarding accept request', acceptRequest, 'email_proof_token: String');
assertNotIncludes('admin onboarding accept request', acceptRequest, 'code: String');
assertNotIncludes('admin onboarding accept request', acceptRequest, 'totp_code');

const reset = read('src/admin_reset.rs');
assertIncludes('admin reset policy', reset, 'public reset request endpoint always returns neutral response');
assertIncludes('admin reset policy', reset, 'superadmin can review all reset requests');
assertIncludes('admin reset policy', reset, 'admin can review moderator reset requests only');
assertIncludes('admin reset policy', reset, 'admin cannot approve admin reset requests');
assertIncludes('admin reset policy', reset, 'moderator cannot review reset requests');
assertIncludes('admin reset neutral response', reset, 'If this email is eligible for admin recovery');
assertIncludes('admin reset reviewer', reset, 'authorize_reset_reviewer');
assertIncludes('admin reset target guard', reset, 'can_review_target');
assertIncludes('admin reset superadmin rule', reset, '"superadmin" => true');
assertIncludes('admin reset admin rule', reset, '"admin" => target_role == Some("moderator")');
assertIncludes('admin reset query scope', reset, "or ($3::text = 'admin' and target.target_role = 'moderator')");
assertNotIncludes('admin reset old broad guard', reset, 'authorize_admin_manage');

const migration = read('migrations/202606200003_admin_invite_only_model.sql');
assertIncludes('admin migration invitations', migration, 'create table if not exists admin_invitations');
assertIncludes('admin migration email otp', migration, 'create table if not exists admin_invite_email_otps');
assertIncludes('admin migration reset', migration, 'create table if not exists admin_reset_requests');
assertIncludes('admin migration token hash', migration, 'token_hash text not null');
assertIncludes('admin migration reset request type', migration, "request_type text not null check (request_type in ('password', 'email', 'totp'))");

console.log('PASS 25E-K backend admin security boundary audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: backend admin invite/onboarding/reset boundaries are locked to superadmin/admin/moderator hierarchy.');
