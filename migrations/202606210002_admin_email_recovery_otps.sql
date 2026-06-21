create table if not exists admin_email_recovery_otps (
  id uuid primary key,
  artifact_id uuid not null references admin_recovery_artifacts(id) on delete cascade,
  reset_request_id uuid not null references admin_reset_requests(id) on delete cascade,
  user_id uuid not null references users(id) on delete cascade,
  old_email text not null,
  new_email text not null,
  otp_hash text not null,
  attempt_count integer not null default 0,
  expires_at timestamptz not null,
  consumed_at timestamptz,
  revoked_at timestamptz,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now()
);

create index if not exists admin_email_recovery_otps_artifact_idx on admin_email_recovery_otps(artifact_id);
create index if not exists admin_email_recovery_otps_new_email_idx on admin_email_recovery_otps(lower(new_email));
create index if not exists admin_email_recovery_otps_status_idx on admin_email_recovery_otps(expires_at, consumed_at, revoked_at);
