# Public Social Media Runtime

**Pass:** PUBLIC-SOCIAL-05, updated by PUBLIC-SOCIAL-10/11  
**Repository:** `spark-api`  
**Scope:** Presigned S3-compatible upload and public media delivery path for the public social layer.

This runtime lets social feed posts attach media without exposing raw bucket URLs in public feed payloads.

## Current public beta storage

The current public beta uses Cloudflare R2 through the S3-compatible API.

```bash
S3_ENDPOINT=https://<ACCOUNT_ID>.r2.cloudflarestorage.com
S3_BUCKET_PUBLIC=spark-public
S3_BUCKET_PRIVATE=spark-private
S3_REGION=us-east-1
S3_PRESIGN_EXPIRES_SECONDS=900
S3_ACCESS_KEY=<r2-access-key-id>
S3_SECRET_KEY=<r2-secret-access-key>
```

MinIO remains a future/self-hosted storage option, not the active public beta storage.

## Runtime routes

```text
GET  /v1/media/policy
GET  /v1/media/public/:asset_id
GET  /v1/media/me/assets
POST /v1/media/upload-intents
GET  /v1/media/assets/:asset_id
POST /v1/media/assets/:asset_id/complete
POST /v1/media/assets/:asset_id/links
```

## Public media delivery

Public media assets store a relative public URL:

```text
/v1/media/public/:asset_id
```

The public delivery endpoint only serves assets where:

```text
status = uploaded
visibility = public
```

The endpoint returns a temporary redirect to a signed object URL when credentials are configured. This lets feed payloads expose Spark API URLs instead of raw R2 or MinIO bucket URLs.

## Upload flow

### 1. Create an upload intent

```bash
curl -s -X POST \
  -H "content-type: application/json" \
  --cookie "spark_session=<session>" \
  -d '{
    "purpose": "community",
    "file_name": "spark-note.png",
    "mime_type": "image/png",
    "size_bytes": 12345,
    "private": false
  }' \
  https://spark.user.cloudjkt01.com/v1/media/upload-intents | jq
```

Response includes:

```json
{
  "upload_method": "PUT",
  "upload_url": "...presigned-url...",
  "presigned": true,
  "asset": {
    "id": "...",
    "status": "pending",
    "public_url": "/v1/media/public/..."
  }
}
```

### 2. Upload file bytes to R2/S3-compatible storage

```bash
curl -X PUT \
  -H "content-type: image/png" \
  --data-binary @spark-note.png \
  "<upload_url>"
```

For browser uploads, configure the bucket CORS policy to allow the Spark web origin, `PUT`, and `content-type`.

### 3. Complete the asset

```bash
curl -s -X POST \
  -H "content-type: application/json" \
  --cookie "spark_session=<session>" \
  -d '{"size_bytes":12345}' \
  https://spark.user.cloudjkt01.com/v1/media/assets/ASSET_UUID/complete | jq
```

### 4. Attach asset to a social post

`POST /v1/social/posts` accepts uploaded public asset ids through `media_asset_ids`.

```bash
curl -s -X POST \
  -H "content-type: application/json" \
  --cookie "spark_session=<session>" \
  -d '{
    "kind": "post",
    "body": "Catatan kecil dari sesi belajar hari ini.",
    "visibility": "community",
    "media_asset_ids": ["ASSET_UUID"]
  }' \
  https://spark.user.cloudjkt01.com/v1/social/posts | jq
```

## Notes

- No new database migration is required for R2.
- The runtime uses path-style S3-compatible URLs: `endpoint/bucket/object_key`.
- The API does not make buckets public by itself.
- Keep R2 credentials in `.env.host`, never in Git and never in frontend environment variables.
- Public social feed should use the `media[].public_url` value from API responses.
- See `docs/CLOUDFLARE_R2_MEDIA_RUNTIME.md` for setup and `docs/PUBLIC_SOCIAL_MEDIA_E2E_VERIFICATION.md` for final verification.
