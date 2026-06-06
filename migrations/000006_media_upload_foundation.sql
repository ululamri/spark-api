-- Karyra Spark API Pass 50: Media upload foundation.
-- Upload intents are persisted as media assets and can be linked to product entities.
-- Storage remains S3-compatible first: MinIO/Garage now, R2/S3 migration later.

alter table media_assets add column if not exists original_file_name text not null default '';
alter table media_assets add column if not exists storage_provider text not null default 's3-compatible';
alter table media_assets add column if not exists upload_method text not null default 'PUT';
alter table media_assets add column if not exists upload_expires_at timestamptz;
alter table media_assets add column if not exists uploaded_at timestamptz;
alter table media_assets add column if not exists public_url text;
alter table media_assets add column if not exists metadata jsonb not null default '{}'::jsonb;
alter table media_assets add column if not exists updated_at timestamptz not null default now();

create index if not exists idx_media_assets_owner_status_created
  on media_assets(owner_user_id, status, created_at desc);

create index if not exists idx_media_assets_visibility_status
  on media_assets(visibility, status, created_at desc);

create index if not exists idx_media_links_entity
  on media_links(entity_type, entity_id, purpose, created_at desc);

create unique index if not exists idx_media_links_unique_asset_entity_purpose
  on media_links(media_asset_id, entity_type, entity_id, purpose);
