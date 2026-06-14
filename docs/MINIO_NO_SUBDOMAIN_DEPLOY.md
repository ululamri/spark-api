# MinIO Deploy Without Subdomain

Use this when the available domain cannot create child domains such as `storage.example.com`.

## Recommended approach

Expose MinIO API on the same hostname with a dedicated HTTPS port:

```text
https://spark.user.cloudjkt01.com:9443
```

This keeps S3 presigned URLs path-style compatible:

```text
https://spark.user.cloudjkt01.com:9443/spark-public/object-key
```

Do not proxy MinIO through a path such as `/minio/*` for the current presigned URL flow. S3 signatures include the canonical URI and bucket/object path. A path prefix can cause signature mismatch or make MinIO interpret the prefix as the bucket name.

## 1. Keep MinIO local

MinIO still listens only on localhost:

```text
127.0.0.1:9000
127.0.0.1:9001
```

Use this in `/etc/default/minio`:

```bash
MINIO_SERVER_URL=https://spark.user.cloudjkt01.com:9443
MINIO_BROWSER_REDIRECT_URL=https://spark.user.cloudjkt01.com/minio-console-disabled
```

## 2. Add Caddy HTTPS port for MinIO API

Add this server block to `/etc/caddy/Caddyfile`:

```caddyfile
spark.user.cloudjkt01.com:9443 {
    encode zstd gzip

    header {
        X-Content-Type-Options nosniff
        Referrer-Policy no-referrer
    }

    reverse_proxy 127.0.0.1:9000 {
        header_up Host {http.request.host}
        header_up X-Real-IP {remote_host}
        header_up X-Forwarded-For {remote_host}
        header_up X-Forwarded-Proto {scheme}
    }
}
```

Validate and reload:

```bash
caddy validate --config /etc/caddy/Caddyfile
systemctl reload caddy
```

Open the firewall for the dedicated TLS port:

```bash
ufw allow 9443/tcp
ufw status
```

Smoke test:

```bash
curl -I https://spark.user.cloudjkt01.com:9443/minio/health/live
```

## 3. Use the port endpoint in Spark API

Set `/opt/karyra/spark-api/.env.host`:

```bash
S3_ENDPOINT=https://spark.user.cloudjkt01.com:9443
S3_BUCKET_PUBLIC=spark-public
S3_BUCKET_PRIVATE=spark-private
S3_REGION=us-east-1
S3_PRESIGN_EXPIRES_SECONDS=900
S3_ACCESS_KEY=REPLACE_WITH_MINIO_ROOT_USER
S3_SECRET_KEY=REPLACE_WITH_MINIO_ROOT_PASSWORD
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

## 4. CORS bucket config

The allowed origin remains the public Spark app origin, not the MinIO port endpoint:

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

Apply it:

```bash
mc cors set karyra-local/spark-public /tmp/spark-minio-cors.json
mc cors set karyra-local/spark-private /tmp/spark-minio-cors.json
mc cors get karyra-local/spark-public
```

## 5. Alternative if port 9443 is blocked

Use an API relay upload mode instead of browser-to-MinIO presigned upload.

That means:

- Browser uploads to Spark API on the normal domain.
- Spark API writes bytes to private MinIO locally.
- Public reads are served through Spark API.

This avoids any public MinIO endpoint but requires a backend code pass and sends file bytes through the Spark API process.
