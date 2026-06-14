# PASS PUBLIC-SOCIAL-10 — Cloudflare R2 Media Runtime

This guide switches Spark media storage to Cloudflare R2 while keeping the existing S3-compatible presigned URL flow.

## Why R2 for this pass

R2 is S3-compatible and avoids the need to operate MinIO during the first public beta. Spark API already signs path-style S3-compatible URLs, so the runtime only needs endpoint, bucket, and access key changes.

## R2 values needed

From Cloudflare Dashboard:

- Account ID
- R2 Access Key ID
- R2 Secret Access Key
- Bucket names

Use this endpoint format:

```text
https://<ACCOUNT_ID>.r2.cloudflarestorage.com
```

Use this region value for Spark compatibility:

```text
us-east-1
```

Cloudflare R2 treats `us-east-1` as an alias for the `auto` region in S3-compatible tools.

## Recommended buckets

Create two private buckets in Cloudflare R2:

```text
spark-public
spark-private
```

Do not enable public bucket access for the first beta. Public social media should be exposed through Spark API redirect URLs:

```text
/v1/media/public/:asset_id
```

## CORS

For browser presigned uploads, set CORS on both buckets.

Use the Cloudflare Dashboard:

```text
R2 Object Storage -> bucket -> Settings -> CORS Policy -> Add CORS policy -> JSON
```

Policy:

```json
[
  {
    "AllowedOrigins": ["https://spark.user.cloudjkt01.com"],
    "AllowedMethods": ["GET", "PUT", "HEAD"],
    "AllowedHeaders": ["*"],
    "ExposeHeaders": ["ETag"],
    "MaxAgeSeconds": 3600
  }
]
```

## Spark API environment

Edit `/opt/karyra/spark-api/.env.host`:

```bash
S3_ENDPOINT=https://<ACCOUNT_ID>.r2.cloudflarestorage.com
S3_BUCKET_PUBLIC=spark-public
S3_BUCKET_PRIVATE=spark-private
S3_REGION=us-east-1
S3_PRESIGN_EXPIRES_SECONDS=900
S3_ACCESS_KEY=<R2_ACCESS_KEY_ID>
S3_SECRET_KEY=<R2_SECRET_ACCESS_KEY>
```

Restart Spark API:

```bash
cd /opt/karyra/spark-api
set -a
source .env.host
set +a
cargo build --release
systemctl restart karyra-spark-api
systemctl status karyra-spark-api --no-pager
```

## Smoke test

Policy endpoint:

```bash
curl -s https://spark.user.cloudjkt01.com/v1/media/policy | jq
```

Browser test:

1. Login to Spark.
2. Open community discussion.
3. Create a text post with one small image or `.txt` file.
4. Confirm the post appears.
5. Confirm media opens through `/v1/media/public/:asset_id`.

## If upload fails

Check these first:

- R2 CORS exact origin is `https://spark.user.cloudjkt01.com` with no trailing slash.
- Allowed methods include `PUT`.
- Allowed headers include `*` or at least `Content-Type`.
- `S3_ENDPOINT` uses Account ID, not bucket name.
- Buckets are named exactly like `.env.host`.
- Spark API was restarted after changing `.env.host`.
