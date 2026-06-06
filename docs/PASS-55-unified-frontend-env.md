# Pass 55 — Unified Frontend Env & Docker Fix

Pass 54 moved Spark toward a same-server unified Docker stack. This pass completes the missing frontend side.

## Same-domain deployment model

Public domain:

```txt
https://spark.user.cloudjkt01.com
```

Routes:

```txt
/          -> spark-web
/v1/*      -> spark-api
/health/*  -> spark-api
```

Because frontend and backend are served under the same public domain, the frontend should prefer relative API paths such as:

```ts
fetch('/v1/auth/me', { credentials: 'include' })
```

For this reason, frontend public API base values should remain blank in staging:

```env
PUBLIC_API_BASE=
PUBLIC_SPARK_API_BASE=
```

## Server setup reminder

Expected folders:

```txt
/opt/karyra/spark
/opt/karyra/spark-api
```

Frontend env:

```bash
cd /opt/karyra/spark
cp .env.unified.staging.example .env.unified.staging
vim .env.unified.staging
```

Backend env:

```bash
cd /opt/karyra/spark-api
cp config/env.unified.staging.example config/env.unified.staging
vim config/env.unified.staging
```

Deploy:

```bash
cd /opt/karyra/spark-api
bash scripts/karyra-unified-stack-deploy.sh config/env.unified.staging
```
