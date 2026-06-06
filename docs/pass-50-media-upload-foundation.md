# Pass 50 — Media Upload Foundation

Pass 50 turns the previous media placeholder into a backend media foundation for Karyra Spark.

## Scope

Implemented now:

- Authenticated upload intent creation.
- Media asset persistence in `media_assets`.
- Upload completion status update.
- Media asset listing for the current user.
- Media link creation through `media_links`.
- S3-compatible storage boundary for MinIO/Garage first.
- Public/private bucket selection using `S3_BUCKET_PUBLIC` and `S3_BUCKET_PRIVATE`.

Held for server/deploy stage:

- Real S3 SigV4 presigned URL generation.
- Actual MinIO/Garage bucket policy setup.
- CDN/public asset delivery hardening.
- Antivirus/content scanning.
- Image transforms/thumbnails.

## Endpoints

```txt
GET  /v1/media/policy
GET  /v1/media/me/assets
GET  /v1/media/assets/{asset_id}
POST /v1/media/upload-intents
POST /v1/media/assets/{asset_id}/complete
POST /v1/media/assets/{asset_id}/links
```

## Notes

The returned `upload_url` is an S3-compatible object URL foundation. In local/self-hosted setups this can point to MinIO/Garage. Production hardening will replace this with real signed upload URLs during server integration.
