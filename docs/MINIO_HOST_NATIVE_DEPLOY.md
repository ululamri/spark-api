# PASS PUBLIC-SOCIAL-10 — MinIO Host-Native Deploy Guide

This guide installs a small host-native MinIO deployment for Karyra Spark public social media uploads.

## Target topology

- MinIO API listens locally on `127.0.0.1:9000`.
- MinIO Console listens locally on `127.0.0.1:9001`.
- Caddy exposes only the S3-compatible API through a dedicated HTTPS hostname.
- Spark API signs PUT/GET URLs against that HTTPS hostname.
- Buckets remain private. Browser access is allowed only through presigned URLs.

Recommended public endpoint placeholder:

```text
https://storage.spark.user.cloudjkt01.com
```

Replace it with the actual DNS name you choose.

## 0. Prepare DNS

Create an A record pointing to the VPS:

```text
storage.spark.user.cloudjkt01.com -> VPS_PUBLIC_IP
```

Wait until it resolves:

```bash
getent hosts storage.spark.user.cloudjkt01.com
```

## 1. Install MinIO and mc

Use the official MinIO Linux AMD64 release endpoints.

```bash
cd /tmp
wget https://dl.min.io/server/minio/release/linux-amd64/minio
wget https://dl.min.io/client/mc/release/linux-amd64/mc

install -m 0755 minio /usr/local/bin/minio
install -m 0755 mc /usr/local/bin/mc

minio --version
mc --version
```

## 2. Create system user and data path

```bash
useradd --system --home /var/lib/minio --shell /usr/sbin/nologin minio-user || true
mkdir -p /var/lib/minio/data
chown -R minio-user:minio-user /var/lib/minio
chmod 750 /var/lib/minio
```

## 3. Create MinIO environment file

Generate strong credentials first:

```bash
openssl rand -base64 24
openssl rand -base64 48
```

Create `/etc/default/minio`:

```bash
cat >/etc/default/minio <<'EOF'
MINIO_ROOT_USER=REPLACE_WITH_STRONG_ROOT_USER
MINIO_ROOT_PASSWORD=REPLACE_WITH_STRONG_ROOT_PASSWORD
MINIO_VOLUMES=/var/lib/minio/data
MINIO_OPTS="--address 127.0.0.1:9000 --console-address 127.0.0.1:9001"
MINIO_SERVER_URL=https://storage.spark.user.cloudjkt01.com
MINIO_BROWSER_REDIRECT_URL=https://storage.spark.user.cloudjkt01.com/minio-console-disabled
EOF

chmod 600 /etc/default/minio
chown root:root /etc/default/minio
```

## 4. Create systemd unit

```bash
cat >/etc/systemd/system/minio.service <<'EOF'
[Unit]
Description=MinIO object storage for Karyra Spark
Documentation=https://min.io/docs/
Wants=network-online.target
After=network-online.target

[Service]
User=minio-user
Group=minio-user
EnvironmentFile=/etc/default/minio
ExecStart=/usr/local/bin/minio server $MINIO_OPTS $MINIO_VOLUMES
Restart=always
RestartSec=5
LimitNOFILE=65536
TasksMax=infinity
TimeoutStopSec=infinity
SendSIGKILL=no

NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=full
ProtectHome=true
ReadWritePaths=/var/lib/minio

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable --now minio
systemctl status minio --no-pager
```

Check local health:

```bash
curl -s http://127.0.0.1:9000/minio/health/live && echo
curl -s http://127.0.0.1:9000/minio/health/ready && echo
```

## 5. Add Caddy public API hostname

Add a new server block to the live Caddyfile:

```caddyfile
storage.spark.user.cloudjkt01.com {
    encode zstd gzip

    header {
        X-Content-Type-Options nosniff
        Referrer-Policy no-referrer
    }

    reverse_proxy 127.0.0.1:9000 {
        header_up Host {host}
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
curl -I https://storage.spark.user.cloudjkt01.com/minio/health/live
```

Do not expose the console until a separate admin-only access path is designed.

## 6. Create buckets

```bash
mc alias set karyra-local http://127.0.0.1:9000 "$MINIO_ROOT_USER" "$MINIO_ROOT_PASSWORD"
mc mb --ignore-existing karyra-local/spark-public
mc mb --ignore-existing karyra-local/spark-private
mc ls karyra-local
```

## 7. Configure CORS for browser presigned upload

Create `/tmp/spark-minio-cors.json`:

```bash
cat >/tmp/spark-minio-cors.json <<'EOF'
[
  {
    "AllowedOrigins": ["https://spark.user.cloudjkt01.com"],
    "AllowedMethods": ["GET", "PUT", "HEAD"],
    "AllowedHeaders": ["*"],
    "ExposeHeaders": ["ETag"],
    "MaxAgeSeconds": 3600
  }
]
EOF
```

Apply to both buckets:

```bash
mc cors set karyra-local/spark-public /tmp/spark-minio-cors.json
mc cors set karyra-local/spark-private /tmp/spark-minio-cors.json
mc cors get karyra-local/spark-public
mc cors get karyra-local/spark-private
```

## 8. Keep buckets private

Do not run `mc anonymous set public` for either bucket.

Check policy:

```bash
mc anonymous get karyra-local/spark-public
mc anonymous get karyra-local/spark-private
```

## 9. Update Spark API `.env.host`

Edit `/opt/karyra/spark-api/.env.host`:

```bash
S3_ENDPOINT=https://storage.spark.user.cloudjkt01.com
S3_BUCKET_PUBLIC=spark-public
S3_BUCKET_PRIVATE=spark-private
S3_REGION=us-east-1
S3_PRESIGN_EXPIRES_SECONDS=900
S3_ACCESS_KEY=REPLACE_WITH_STRONG_ROOT_USER
S3_SECRET_KEY=REPLACE_WITH_STRONG_ROOT_PASSWORD
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

## 10. Smoke test

Policy endpoint:

```bash
curl -s https://spark.user.cloudjkt01.com/v1/media/policy | jq
```

Create an upload intent while logged in through browser, or use a session cookie:

```bash
curl -s -X POST \
  -H 'content-type: application/json' \
  --cookie 'spark_session=REPLACE_WITH_SESSION_COOKIE' \
  -d '{
    "purpose": "community",
    "file_name": "smoke.txt",
    "mime_type": "text/plain",
    "size_bytes": 12,
    "private": false
  }' \
  https://spark.user.cloudjkt01.com/v1/media/upload-intents | jq
```

Upload test bytes to the returned `upload_url`:

```bash
printf 'hello spark\n' >/tmp/smoke.txt
curl -X PUT -H 'content-type: text/plain' --data-binary @/tmp/smoke.txt 'PASTE_UPLOAD_URL_HERE'
```

Complete the asset:

```bash
curl -s -X POST \
  -H 'content-type: application/json' \
  --cookie 'spark_session=REPLACE_WITH_SESSION_COOKIE' \
  -d '{"size_bytes":12}' \
  https://spark.user.cloudjkt01.com/v1/media/assets/ASSET_UUID/complete | jq
```

Public media redirect should work for uploaded public assets:

```bash
curl -I https://spark.user.cloudjkt01.com/v1/media/public/ASSET_UUID
```

## Rollback

```bash
systemctl stop minio
systemctl disable minio
rm -f /etc/systemd/system/minio.service
systemctl daemon-reload
```

Keep `/var/lib/minio/data` until you are sure no uploaded assets need to be preserved.
