# Pass 54 — Unified Docker Stack for Spark Web + API

This pass prepares Karyra Spark for a same-server staging deployment.

The intended public layout is:

```txt
https://spark.user.cloudjkt01.com/         -> SvelteKit frontend
https://spark.user.cloudjkt01.com/v1/*     -> Rust/Axum Spark API
https://spark.user.cloudjkt01.com/health/* -> Rust/Axum Spark API
```

## Why one stack?

One Compose stack is easier for staging and grant readiness:

- one Dockge stack to inspect
- one network for frontend, API, PostgreSQL, MinIO, and Caddy
- service-level restart remains possible
- frontend failure does not automatically stop backend
- backend failure does not automatically stop PostgreSQL or MinIO

Each service is still isolated:

```txt
caddy      = reverse proxy + HTTPS
spark-web  = frontend container
spark-api  = backend container
postgres   = database container
minio      = object storage container
```

## Files added

Backend repo:

```txt
infra/docker-compose.unified.staging.yml
infra/caddy/Caddyfile
config/env.unified.staging.example
scripts/karyra-pass54-unified-stack-audit.sh
scripts/karyra-unified-stack-env-audit.sh
scripts/karyra-unified-stack-deploy.sh
scripts/karyra-unified-stack-smoke.sh
scripts/karyra-unified-stack-logs.sh
scripts/karyra-unified-stack-restart-service.sh
docs/PASS_54_UNIFIED_DOCKER_STACK.md
```

Frontend repo:

```txt
Dockerfile.staging
```

## Install

From the pass folder:

```bash
python3 install_pass_54_unified_docker_stack.py --backend-root ~/spark-api --frontend-root ~/spark
```

## Local lightweight check

```bash
cd ~/spark-api
bash scripts/karyra-pass54-unified-stack-audit.sh
cargo fmt --check
cargo check
cargo build
```

## Server preparation

On the server, keep frontend and backend as sibling repos when possible:

```txt
/opt/karyra/spark
/opt/karyra/spark-api
```

Copy env:

```bash
cd /opt/karyra/spark-api
cp config/env.unified.staging.example config/env.unified.staging
vim config/env.unified.staging
```

Replace every `CHANGE_ME` value.

Recommended values for same-server HTTPS staging:

```env
SPARK_DOMAIN=spark.user.cloudjkt01.com
SPARK_WEB_ORIGIN=https://spark.user.cloudjkt01.com
SPARK_COOKIE_SECURE=true
S3_ENDPOINT=http://minio:9000
DATABASE_URL=postgres://spark:<password>@postgres:5432/spark
```

## Deploy

```bash
cd /opt/karyra/spark-api
bash scripts/karyra-unified-stack-deploy.sh config/env.unified.staging
```

## Smoke test

```bash
bash scripts/karyra-unified-stack-smoke.sh https://spark.user.cloudjkt01.com
```

## Service-level restart

Do not restart the whole stack when only one service changes.

Frontend only:

```bash
bash scripts/karyra-unified-stack-restart-service.sh config/env.unified.staging spark-web
```

Backend only:

```bash
bash scripts/karyra-unified-stack-restart-service.sh config/env.unified.staging spark-api
```

Logs:

```bash
bash scripts/karyra-unified-stack-logs.sh config/env.unified.staging spark-api
bash scripts/karyra-unified-stack-logs.sh config/env.unified.staging spark-web
bash scripts/karyra-unified-stack-logs.sh config/env.unified.staging caddy
```

## Dockge usage

Dockge can manage this stack by pointing it to:

```txt
/opt/karyra/spark-api/infra/docker-compose.unified.staging.yml
```

Use the env file:

```txt
/opt/karyra/spark-api/config/env.unified.staging
```

Keep Dockge private. Prefer SSH tunnel instead of exposing the panel publicly.

## Important notes

This pass does not add product features. It only prepares same-server staging deployment.

Caddy will request HTTPS certificates automatically when the domain points to the server and ports 80/443 are reachable.

PostgreSQL and MinIO are not exposed publicly in this stack. They are only reachable inside the Docker network.
