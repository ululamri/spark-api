# Karyra Spark Directus Experiment

This is an isolated experiment for the future Karyra Content Studio.

Scope:

- Learn CMS authoring.
- Lab CMS authoring.
- AI review records.
- Draft/review/publish workflow data.

Non-scope:

- Do not manage Spark users here.
- Do not manage sessions here.
- Do not manage social runtime tables here.
- Do not manage proof ledger or passport credentials here.
- Do not replace `/admin` until this experiment is proven.

## Intended server path

Use a new isolated path:

```bash
/opt/karyra/directus
```

Do not use old or wrong paths such as `/home/spark` or `/home/spark-api`.

## Preflight

Run this first on the live server:

```bash
docker --version
docker compose version
```

If Docker Compose is not installed, stop here and decide whether Directus will run through Docker or native Node/systemd.

## Install experiment files

From the backend repo after pulling the latest commit:

```bash
cd /opt/karyra/spark-api
git pull --ff-only

mkdir -p /opt/karyra/directus
cp -a ops/directus-experiment/docker-compose.yml /opt/karyra/directus/docker-compose.yml
cp -a ops/directus-experiment/directus.env.example /opt/karyra/directus/directus.env
mkdir -p /opt/karyra/directus/schema
cp -a ops/directus-experiment/schema/karyra_learn_lab_schema.sql /opt/karyra/directus/schema/karyra_learn_lab_schema.sql
```

Edit secrets:

```bash
cd /opt/karyra/directus
nano directus.env
```

Generate strong values:

```bash
openssl rand -base64 48
openssl rand -base64 48
openssl rand -base64 32
```

Set at least:

```env
SECRET=<strong random value>
KEY=<strong random value>
ADMIN_EMAIL=<private admin email>
ADMIN_PASSWORD=<strong password>
DIRECTUS_DB_PASSWORD=<strong random database password>
PUBLIC_URL=http://127.0.0.1:8055
```

## Start isolated Directus

```bash
cd /opt/karyra/directus
set -a
source directus.env
set +a

docker compose --env-file directus.env up -d
```

Check:

```bash
docker ps --filter name=karyra-directus
curl -I http://127.0.0.1:8055/server/health || true
```

## Apply Learn/Lab schema

```bash
cd /opt/karyra/directus
set -a
source directus.env
set +a

cat schema/karyra_learn_lab_schema.sql | docker exec -i karyra-directus-db \
  psql -U "$DIRECTUS_DB_USER" -d "$DIRECTUS_DB_DATABASE"
```

After this, open Directus locally or through a temporary protected tunnel and verify the collections exist.

## Experiment safety rules

- Keep Directus bound to `127.0.0.1:8055` until Caddy routing is reviewed.
- Do not expose Directus publicly without TLS, auth, and route policy.
- Keep Directus DB isolated from the Spark API production DB for this phase.
- Spark public frontend must not call Directus directly.
- Spark API will later read published content through a controlled bridge.

## PASS sequence

```txt
PASS 24A — freeze custom admin as fallback and validate build
PASS 24B — isolated Directus experiment setup
PASS 24C — Learn/Lab schema verification
PASS 24D — Spark API bridge for published content
PASS 24E — Admin AI review workflow
PASS 24F — Learn/Lab frontend renderer contract
PASS 24G — replace-or-fallback decision
```
