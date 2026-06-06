# Pass 56 — Provider-Managed HTTPS Mode

Spark staging currently uses a same-server unified Docker stack, but public HTTPS is terminated by the hosting provider before traffic reaches the server.

This means Caddy should not request or serve public TLS certificates directly. Caddy should act as an internal HTTP reverse proxy:

- `/` -> `spark-web:4173`
- `/v1/*` -> `spark-api:8787`
- `/health/*` -> `spark-api:8787`

Public browser traffic still uses:

```txt
https://spark.user.cloudjkt01.com
```

Inside the server, validation should prefer Host-header HTTP checks:

```bash
bash scripts/karyra-provider-https-smoke.sh spark.user.cloudjkt01.com
```

The runtime env should keep:

```env
SPARK_WEB_ORIGIN=https://spark.user.cloudjkt01.com
SPARK_COOKIE_SECURE=true
SPARK_HTTPS_MODE=provider-managed
```

Do not expose PostgreSQL, MinIO, Dockge, or internal service ports publicly.

If the hosting provider is later changed to direct DNS-to-server mode, Caddy can be switched back to direct TLS mode by removing `auto_https off` and using `{$SPARK_DOMAIN}` as the site address.
