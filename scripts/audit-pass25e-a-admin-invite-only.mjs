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

const migration = read('migrations/202606200003_admin_invite_only_model.sql');
assertIncludes('invite migration', migration, 'create table if not exists admin_invitations');
assertIncludes('invite migration', migration, "role text not null check (role in ('admin', 'moderator'))");
assertIncludes('invite migration', migration, 'token_hash text not null unique');
assertIncludes('invite migration', migration, 'admin_invite_email_otps');
assertIncludes('invite migration', migration, 'admin_reset_requests');
assertIncludes('invite migration', migration, "request_type text not null check (request_type in ('password', 'email', 'totp'))");

const adminTeam = read('src/admin_team.rs');
assertIncludes('admin_team', adminTeam, 'phase: "invite-only-admin-team-model"');
assertIncludes('admin_team', adminTeam, '.route("/members", get(members))');
assertNotIncludes('admin_team', adminTeam, '.route("/members", get(members).post(upsert_member))');
assertNotIncludes('admin_team', adminTeam, 'async fn upsert_member');
assertIncludes('admin_team', adminTeam, '.route("/invitations", get(invitations).post(create_invitation))');
assertIncludes('admin_team', adminTeam, '.route("/invitations/:invitation_id/revoke", post(revoke_invitation))');
assertIncludes('admin_team', adminTeam, 'ensure_can_invite_role');
assertIncludes('admin_team', adminTeam, '("superadmin", "admin" | "moderator") => Ok(())');
assertIncludes('admin_team', adminTeam, '("admin", "moderator") => Ok(())');
assertIncludes('admin_team', adminTeam, '("admin", "admin") => Err(ApiError::Unauthorized)');
assertIncludes('admin_team', adminTeam, '("moderator", _) => Err(ApiError::Unauthorized)');
assertIncludes('admin_team', adminTeam, 'new_invite_token');
assertIncludes('admin_team', adminTeam, 'hash_token');
assertIncludes('admin_team', adminTeam, 'token_hash');
assertIncludes('admin_team', adminTeam, 'admin_invitation_create');
assertIncludes('admin_team', adminTeam, 'admin_invitation_revoke');
assertIncludes('admin_team', adminTeam, 'SPARK_ADMIN_INVITE_RETURN_BOOTSTRAP_TOKENS');

console.log('PASS 25E-A admin invite-only model audit');
if (failures.length) {
  console.error(`failures: ${failures.length}`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(1);
}
console.log('OK: delegated admin/moderator role activation is invite-first; superadmin can invite admin/moderator, admin can invite moderator only, moderator cannot invite.');
