# PASS PUBLIC-SOCIAL-11 — Public Social Media E2E Verification

This checklist verifies that public social media upload works end to end with Cloudflare R2 as the current object storage runtime.

## Current runtime

- Storage provider: Cloudflare R2 through S3-compatible API.
- Spark API media endpoint: `/v1/media/*`.
- Public media delivery path: `/v1/media/public/:asset_id`.
- Browser upload path: presigned `PUT` directly to R2.
- Buckets: `spark-public`, `spark-private`.
- Buckets remain private.

## 1. API health

```bash
curl -s https://spark.user.cloudjkt01.com/health/live && echo
curl -s https://spark.user.cloudjkt01.com/health/ready | jq
curl -s https://spark.user.cloudjkt01.com/v1/media/policy | jq
```

Expected:

- health endpoints return healthy JSON.
- media policy returns `current_phase: presigned-media-runtime`.
- no object storage credentials appear in the response.

## 2. Social feed media hydration

```bash
curl -s https://spark.user.cloudjkt01.com/v1/social/feed | jq '.items[0] | {post: .post.id, media: .media}'
```

Expected for a post with media:

```json
{
  "post": "...",
  "media": [
    {
      "id": "...",
      "original_file_name": "...",
      "mime_type": "image/png",
      "size_bytes": 12345,
      "public_url": "/v1/media/public/..."
    }
  ]
}
```

The feed must expose Spark API media paths, not raw R2 object URLs and not storage credentials.

## 3. Public media redirect

From a media item returned by the feed, test:

```bash
curl -I https://spark.user.cloudjkt01.com/v1/media/public/ASSET_UUID
```

Expected:

- HTTP redirect response.
- `location` points to a temporary signed R2 URL.
- response does not expose R2 secret key.

## 4. Browser upload test

1. Login to Spark.
2. Open community discussion.
3. Create a text post with one small `.txt` file or image.
4. Confirm the post appears after submit.
5. Refresh the page.
6. Confirm the post and media still appear.
7. Open the media link.
8. Confirm the object exists in Cloudflare R2 dashboard.

## 5. API logs

```bash
journalctl -u karyra-spark-api -n 120 --no-pager
```

Expected:

- no repeated `media` errors.
- no presign errors.
- no CORS errors in backend logs.

Browser-side CORS errors appear in DevTools, not in API logs. If browser upload fails while curl works, re-check bucket CORS.

## 6. R2 configuration reminders

Spark API `.env.host` should use:

```bash
S3_ENDPOINT=https://<ACCOUNT_ID>.r2.cloudflarestorage.com
S3_BUCKET_PUBLIC=spark-public
S3_BUCKET_PRIVATE=spark-private
S3_REGION=us-east-1
S3_PRESIGN_EXPIRES_SECONDS=900
S3_ACCESS_KEY=<R2_ACCESS_KEY_ID>
S3_SECRET_KEY=<R2_SECRET_ACCESS_KEY>
```

Do not expose these values in frontend environment variables.

## 7. Release status

Public social media is considered beta-ready when all checks pass:

- text-only post works.
- media post works.
- file is visible in Cloudflare R2.
- media appears after page refresh.
- `/v1/social/feed` includes media metadata.
- `/v1/media/public/:asset_id` redirects successfully.
- no object storage credential appears in API responses or frontend bundle.
