# Karyra Spark imgproxy Rollout Commands

This file is an operator checklist. Do not enable optimized media until every smoke test passes.

## 1. Pull latest backend

```bash
cd /opt/karyra/spark-api
git pull
```

## 2. Generate server-only signing secrets

imgproxy production URLs should be signed with a key and salt. Generate separate values:

```bash
sudo mkdir -p /etc/karyra
sudo sh -c 'printf "IMGPROXY_KEY=" > /etc/karyra/imgproxy.env'
openssl rand -hex 32 | sudo tee -a /etc/karyra/imgproxy.env >/dev/null
sudo sh -c 'printf "IMGPROXY_SALT=" >> /etc/karyra/imgproxy.env'
openssl rand -hex 32 | sudo tee -a /etc/karyra/imgproxy.env >/dev/null
```

Then append runtime options:

```bash
sudo tee -a /etc/karyra/imgproxy.env >/dev/null <<'EOF'
IMGPROXY_BIND=:8088
IMGPROXY_USE_ETAG=true
IMGPROXY_ENABLE_WEBP_DETECTION=true
IMGPROXY_ENABLE_AVIF_DETECTION=true
IMGPROXY_MAX_SRC_RESOLUTION=80
IMGPROXY_MAX_SRC_FILE_SIZE=12582912
IMGPROXY_DOWNLOAD_TIMEOUT=10
IMGPROXY_READ_REQUEST_TIMEOUT=10
IMGPROXY_WRITE_RESPONSE_TIMEOUT=10
IMGPROXY_TTL=2592000
IMGPROXY_CACHE_CONTROL_PASSTHROUGH=false
EOF
```

Keep `/etc/karyra/imgproxy.env` private:

```bash
sudo chmod 600 /etc/karyra/imgproxy.env
sudo chown root:root /etc/karyra/imgproxy.env
```

## 3. Install imgproxy binary

Install `imgproxy` to:

```text
/usr/local/bin/imgproxy
```

Then verify:

```bash
/usr/local/bin/imgproxy -version || /usr/local/bin/imgproxy --version || true
```

## 4. Create service user

```bash
sudo useradd --system --no-create-home --shell /usr/sbin/nologin imgproxy 2>/dev/null || true
```

## 5. Install systemd unit

```bash
sudo cp /opt/karyra/spark-api/deploy/karyra-imgproxy.service.example /etc/systemd/system/karyra-imgproxy.service
sudo systemctl daemon-reload
sudo systemctl enable --now karyra-imgproxy
sudo systemctl status karyra-imgproxy --no-pager
```

## 6. Local smoke test

```bash
curl -i -sS http://127.0.0.1:8088/health | head -40
```

If `/health` is not available in the installed imgproxy build, check service logs instead:

```bash
journalctl -u karyra-imgproxy -n 80 --no-pager
```

## 7. Add Spark API env values

Copy the same hex values into `/opt/karyra/spark-api/.env.host` using the backend env names:

```bash
sudo grep '^IMGPROXY_KEY=' /etc/karyra/imgproxy.env | sed 's/^IMGPROXY_KEY=/IMGPROXY_KEY_HEX=/' | sudo tee -a /opt/karyra/spark-api/.env.host >/dev/null
sudo grep '^IMGPROXY_SALT=' /etc/karyra/imgproxy.env | sed 's/^IMGPROXY_SALT=/IMGPROXY_SALT_HEX=/' | sudo tee -a /opt/karyra/spark-api/.env.host >/dev/null
```

Keep optimizer disabled for the first restart:

```bash
cat <<'EOF' | sudo tee -a /opt/karyra/spark-api/.env.host >/dev/null
SPARK_MEDIA_OPTIMIZER_ENABLED=false
IMGPROXY_PUBLIC_BASE_URL=/media/optimized
IMGPROXY_SOURCE_BASE_URL=https://spark.user.cloudjkt01.com
EOF
```

Restart backend:

```bash
cd /opt/karyra/spark-api
set -a
source .env.host
set +a
cargo build --release
systemctl restart karyra-spark-api
systemctl status karyra-spark-api --no-pager
```

## 8. Backend smoke test while disabled

```bash
curl -i -sS https://spark.user.cloudjkt01.com/v1/media-optimizer/scope | head -100
```

Expected:

```text
enabled: false
key_configured: true
salt_configured: true
```

## 9. Enable Caddy optimized media route

Only after imgproxy service is healthy, uncomment the `/media/optimized/*` block in live `/etc/caddy/Caddyfile`:

```caddy
handle /media/optimized/* {
	header Cache-Control "public, max-age=2592000, stale-while-revalidate=86400"
	reverse_proxy 127.0.0.1:8088
}
```

Then:

```bash
sudo caddy validate --config /etc/caddy/Caddyfile
sudo systemctl reload caddy
sudo systemctl status caddy --no-pager
```

## 10. Enable backend optimizer flag

Edit `/opt/karyra/spark-api/.env.host`:

```text
SPARK_MEDIA_OPTIMIZER_ENABLED=true
```

Restart backend:

```bash
cd /opt/karyra/spark-api
set -a
source .env.host
set +a
systemctl restart karyra-spark-api
systemctl status karyra-spark-api --no-pager
```

## 11. Public asset smoke test

Pick a known public uploaded image asset ID, then:

```bash
curl -i -sS https://spark.user.cloudjkt01.com/v1/media-optimizer/public/<asset-id>/urls | head -160
```

Expected:

```text
optimized_urls.avatar_64
optimized_urls.feed_480
optimized_urls.feed_720
optimized_urls.detail_1080
```

Open one optimized URL in a browser. It should return an image response.

## Rollback

```bash
# Backend disable
sudo sed -i 's/^SPARK_MEDIA_OPTIMIZER_ENABLED=.*/SPARK_MEDIA_OPTIMIZER_ENABLED=false/' /opt/karyra/spark-api/.env.host
systemctl restart karyra-spark-api

# Caddy disable: comment /media/optimized/* block again, then reload
sudo caddy validate --config /etc/caddy/Caddyfile
sudo systemctl reload caddy

# Service stop if needed
sudo systemctl stop karyra-imgproxy
```
