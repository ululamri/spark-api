-- Karyra Spark API Pass 49: Passport Credential API.
-- Passport is a readiness credential derived from system proof records only.
-- This remains backend-only and NFT-ready; Starknet anchoring/minting stays grant scope.

alter table passport_credentials add column if not exists evidence_event_count bigint not null default 0;
alter table passport_credentials add column if not exists schema_version text not null default 'spark-passport-v1';
alter table passport_credentials add column if not exists issuer text not null default 'karyra-spark-api';
alter table passport_credentials add column if not exists credential_hash text;
alter table passport_credentials add column if not exists issued_at timestamptz;
alter table passport_credentials add column if not exists revoked_at timestamptz;
alter table passport_credentials add column if not exists payload jsonb not null default '{}'::jsonb;

create index if not exists idx_passport_credentials_user_status_created
  on passport_credentials(user_id, issue_status, created_at desc);

create index if not exists idx_passport_credentials_evidence_root
  on passport_credentials(evidence_root)
  where evidence_root is not null;

create unique index if not exists idx_passport_credentials_user_issued_unique
  on passport_credentials(user_id)
  where issue_status = 'issued';
