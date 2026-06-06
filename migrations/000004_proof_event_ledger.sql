-- Karyra Spark API Pass 48: Proof Event Ledger backend.
-- Proof events are system-generated evidence records. They are not manual user claims.
-- The latest event_hash per user acts as a backend evidence root for Passport readiness.

alter table proof_events add column if not exists track text;
alter table proof_events add column if not exists source_table text;
alter table proof_events add column if not exists source_id uuid;
alter table proof_events add column if not exists schema_version text not null default 'spark-proof-event-v1';
alter table proof_events add column if not exists issuer text not null default 'karyra-spark-api';
alter table proof_events add column if not exists evidence_root text;

create index if not exists idx_proof_events_user_hash_chain on proof_events(user_id, created_at desc, id desc);
create index if not exists idx_proof_events_user_event_type on proof_events(user_id, event_type, created_at desc);
create index if not exists idx_proof_events_source on proof_events(source_table, source_id);
create unique index if not exists idx_proof_events_unique_source_event
  on proof_events(user_id, source_table, source_id, event_type)
  where source_table is not null and source_id is not null;
