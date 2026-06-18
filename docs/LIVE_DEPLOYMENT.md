# Spark API live deployment

Development repo: `ululamri/spark-api`.

## Live contract

| Item | Value |
|---|---|
| Path | `/opt/karyra/spark-api` |
| Env file | `/opt/karyra/spark-api/.env.host` |
| Service | `karyra-spark-api` |
| Listen address | `127.0.0.1:8787` |
| Public domain | `https://spark.user.cloudjkt01.com` |

## Production env checklist

The live `.env.host` must use production values:

- `APP_ENV=production`
- `SPARK_API_HOST=127.0.0.1`
- `SPARK_API_PORT=8787`
- `SPARK_WEB_ORIGIN=https://spark.user.cloudjkt01.com`
- `SPARK_COOKIE_SECURE=true`

## Deploy command

```bash
cd /opt/karyra/spark-api
git pull
set -a
source .env.host
set +a
cargo build --release
systemctl restart karyra-spark-api
systemctl status karyra-spark-api --no-pager
curl -fsS http://127.0.0.1:8787/health/live
curl -fsS http://127.0.0.1:8787/health/ready
```

## Readiness rule

A successful process restart is not enough. The live backend is ready only when `/health/ready` succeeds after restart.
