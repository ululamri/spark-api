# Karyra Spark Host-Native Deployment Guide

This document explains the current host-native deployment model for Karyra Spark staging.

It is written for maintainers and contributors who need to understand how the public demo is served, how the backend is connected, and how to safely operate the deployment without relying on Docker.

## Deployment summary

Karyra Spark currently runs as a small host-native deployment:

| Component | Runtime | Local address | Public route |
| --- | --- | --- | --- |
| Spark frontend | Node.js systemd service | `127.0.0.1:4173` | `/` |
| Spark API | Rust systemd service | `127.0.0.1:8787` | `/health/*`, `/v1/*`, `/api/admin/*` when routed internally by frontend |
| Spark Hub | Static files served by Caddy | `/opt/karyra/hub/build` | `/hub/*` |
| PostgreSQL | Host service | `127.0.0.1:5432` | Internal only |
| Caddy | Host reverse proxy + HTTPS | `:80`, `:443` | `https://spark.user.cloudjkt01.com` |

Docker configuration may still exist in the repository, but the current staging deployment does not depend on Docker. This keeps the server easier to debug and helps reduce disk usage on small VPS instances.

## Directory layout

```text
/opt/karyra/
├── spark/          # Spark frontend
├── spark-api/      # Rust API and database migrations
├── hub/            # Spark Hub static site
└── backups/        # Manual database and environment backups
```

## Required services

The deployment expects these host services to be enabled:

```bash
systemctl enable postgresql
systemctl enable caddy
systemctl enable karyra-spark-web
systemctl enable karyra-spark-api
```

Check service status:

```bash
systemctl status postgresql --no-pager
systemctl status caddy --no-pager
systemctl status karyra-spark-web --no-pager
systemctl status karyra-spark-api --no-pager
```

## Environment files

The deployment uses private environment files on the server. These files must not be committed to Git.

### Spark frontend

Path:

```text
/opt/karyra/spark/.env
```

Important values:

```env
PUBLIC_SPARK_APP_NAME=Karyra Spark
PUBLIC_SPARK_MODE=production
PUBLIC_SPARK_APP_URL=https://spark.user.cloudjkt01.com
PUBLIC_SPARK_API_URL=/api
PUBLIC_SPARK_HUB_URL=/hub

KARYRA_ADMIN_ENABLED=true
KARYRA_ADMIN_PASSWORD=<admin-password-minimum-12-characters>
KARYRA_ADMIN_SESSION_SECRET=<random-secret-minimum-32-characters>
KARYRA_ADMIN_SESSION_HOURS=8

KARYRA_ADMIN_API_BASE_URL=http://127.0.0.1:8787
KARYRA_ADMIN_TOKEN=<same-token-as-spark-api>
```

The admin password and session secret are read by the Node server at runtime.

### Spark API

Path:

```text
/opt/karyra/spark-api/.env.host
```

Important values:

```env
APP_ENV=production
RUST_LOG=spark_api=info,tower_http=info

SPARK_PUBLIC_ORIGIN=https://spark.user.cloudjkt01.com
SPARK_WEB_ORIGIN=https://spark.user.cloudjkt01.com

SPARK_API_HOST=127.0.0.1
SPARK_API_PORT=8787

DATABASE_URL=postgres://spark:<database-password>@127.0.0.1:5432/spark
DATABASE_MAX_CONNECTIONS=5

SPARK_SESSION_COOKIE=spark_session
SPARK_SESSION_TTL_DAYS=14
SPARK_COOKIE_SECURE=true

KARYRA_ADMIN_TOKEN=<same-token-as-spark-frontend>
```

The admin token is used by the frontend server to call private admin API endpoints.

### Spark Hub

Path:

```text
/opt/karyra/hub/.env
```

Important values:

```env
NODE_ENV=production
PUBLIC_HUB_BASE_PATH=/hub
PUBLIC_SPARK_APP_URL=https://spark.user.cloudjkt01.com
PUBLIC_SPARK_HUB_URL=https://spark.user.cloudjkt01.com/hub/
```

## Build commands

### Spark frontend

```bash
cd /opt/karyra/spark
pnpm install
pnpm run build
systemctl restart karyra-spark-web
```

### Spark API

```bash
cd /opt/karyra/spark-api
source "$HOME/.cargo/env"
cargo build --release
systemctl restart karyra-spark-api
```

### Spark Hub

The Hub uses SvelteKit static output. It is not started with `node build`.

```bash
cd /opt/karyra/hub
pnpm install
pnpm run build
systemctl restart caddy
```

Caddy serves the generated files from:

```text
/opt/karyra/hub/build
```

## Database migrations

Database schema changes must be added to the `migrations/` directory in `spark-api`.

Run migrations from the API repository:

```bash
cd /opt/karyra/spark-api
set -a
source .env.host
set +a
sqlx migrate run
```

Check migration status:

```bash
sqlx migrate info
```

After running migrations, restart the API:

```bash
systemctl restart karyra-spark-api
```

## Caddy routing

Caddy handles public HTTPS and routes requests to the local services.

Path:

```text
/etc/caddy/Caddyfile
```

Example structure:

```caddy
spark.user.cloudjkt01.com {
        encode zstd gzip

        handle /v1/* {
                reverse_proxy 127.0.0.1:8787
        }

        handle /health/* {
                reverse_proxy 127.0.0.1:8787
        }

        handle /health {
                reverse_proxy 127.0.0.1:8787
        }

        redir /hub /hub/ 308

        handle_path /hub/* {
                root * /opt/karyra/hub/build
                try_files {path} {path}/ /index.html
                file_server
        }

        handle {
                reverse_proxy 127.0.0.1:4173
        }

        header {
                X-Content-Type-Options nosniff
                Referrer-Policy strict-origin-when-cross-origin
                -Server
        }
}
```

Validate and reload Caddy:

```bash
caddy validate --config /etc/caddy/Caddyfile
systemctl reload caddy
```

Use `systemctl restart caddy` if reload is not enough after larger changes.

## Health checks

Run these checks after each deployment:

```bash
curl -I https://spark.user.cloudjkt01.com
curl -s https://spark.user.cloudjkt01.com/health/live && echo
curl -I https://spark.user.cloudjkt01.com/hub/
```

Check API locally:

```bash
curl -s http://127.0.0.1:8787/health/live && echo
```

Check admin API locally:

```bash
API_TOKEN=$(grep '^KARYRA_ADMIN_TOKEN=' /opt/karyra/spark-api/.env.host | cut -d= -f2-)

curl -s \
  -H "x-karyra-admin-token: $API_TOKEN" \
  http://127.0.0.1:8787/api/admin/system && echo

curl -s \
  -H "x-karyra-admin-token: $API_TOKEN" \
  http://127.0.0.1:8787/api/admin/overview && echo
```

## Admin access

The private admin interface is available at:

```text
https://spark.user.cloudjkt01.com/admin
```

Admin access depends on these frontend environment values:

```text
KARYRA_ADMIN_ENABLED
KARYRA_ADMIN_PASSWORD
KARYRA_ADMIN_SESSION_SECRET
KARYRA_ADMIN_API_BASE_URL
KARYRA_ADMIN_TOKEN
```

If the login page says admin access is disabled, check that:

- `KARYRA_ADMIN_ENABLED` is `true`.
- `KARYRA_ADMIN_PASSWORD` is at least 12 characters.
- `KARYRA_ADMIN_SESSION_SECRET` is at least 32 characters.
- The frontend service has been restarted after editing `.env`.

## Backup

Create a database backup:

```bash
mkdir -p /opt/karyra/backups

PGPASSWORD='<database-password>' pg_dump -h 127.0.0.1 -U spark -d spark \
  > /opt/karyra/backups/spark_db_$(date +%Y%m%d_%H%M%S).sql
```

Backup deployment configuration:

```bash
tar -czf /opt/karyra/backups/karyra_host_env_$(date +%Y%m%d_%H%M%S).tar.gz \
  /opt/karyra/spark/.env \
  /opt/karyra/spark-api/.env.host \
  /opt/karyra/hub/.env \
  /etc/caddy/Caddyfile \
  /etc/systemd/system/karyra-spark-web.service \
  /etc/systemd/system/karyra-spark-api.service
```

Store backups securely. Environment backups contain secrets.

## Security notes

- Do not commit `.env`, `.env.host`, database dumps, or backup archives.
- Keep PostgreSQL bound to localhost unless remote database access is intentionally required.
- Keep the API bound to `127.0.0.1` and expose only the necessary routes through Caddy.
- Rotate `KARYRA_ADMIN_TOKEN` if it may have been exposed.
- Use HTTPS for the public domain.
- Keep the admin interface private and do not publish credentials in issue threads, docs, screenshots, or commits.

## Troubleshooting

### Frontend is not reachable

```bash
systemctl status karyra-spark-web --no-pager
journalctl -u karyra-spark-web --no-pager -n 120
curl -I http://127.0.0.1:4173
```

### API is not reachable

```bash
systemctl status karyra-spark-api --no-pager
journalctl -u karyra-spark-api --no-pager -n 160
curl -s http://127.0.0.1:8787/health/live && echo
```

### Admin dashboard cannot read data

Check the admin token and database migrations:

```bash
cd /opt/karyra/spark-api
set -a
source .env.host
set +a
sqlx migrate info
```

Then test the admin API:

```bash
API_TOKEN=$(grep '^KARYRA_ADMIN_TOKEN=' /opt/karyra/spark-api/.env.host | cut -d= -f2-)

curl -i \
  -H "x-karyra-admin-token: $API_TOKEN" \
  http://127.0.0.1:8787/api/admin/overview
```

### Hub page is blank

Rebuild the Hub and check Caddy static routing:

```bash
cd /opt/karyra/hub
pnpm run build
find build -maxdepth 3 -type f | head -80
systemctl restart caddy
```

## Release checklist

Before considering a staging update complete:

- [ ] Frontend builds successfully.
- [ ] API builds successfully.
- [ ] Database migrations run successfully.
- [ ] Caddy config validates successfully.
- [ ] Public homepage loads over HTTPS.
- [ ] `/health/live` returns a healthy response.
- [ ] `/hub/` loads correctly.
- [ ] Admin login works.
- [ ] Admin overview can read the backend API.
- [ ] Database backup has been created.
- [ ] Relevant code and migration changes have been pushed to GitHub.
