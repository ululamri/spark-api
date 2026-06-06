# Pass 52 — Server Runtime Hardening

Pass 52 is a safety pass for the backend after the initial foundation batch. It does not add product features. It gives Spark API a clearer path for server-side runtime validation.

## Current backend baseline

The backend foundation now has:

- Auth/session API
- Learning progress API
- Lab attempt API
- Proof event ledger
- Evidence root read model
- Passport credential API
- Media upload foundation
- Docker/server readiness files

## What this pass adds

- `scripts/karyra-pass52-server-runtime-hardening-audit.sh`
- `scripts/karyra-server-env-audit.sh`
- `scripts/karyra-server-migration-check.sh`
- `scripts/karyra-server-api-smoke.sh`
- `scripts/karyra-server-auth-proof-passport-smoke.sh`
- `docs/server/SERVER_RUNTIME_CHECKLIST.md`

## Local lightweight validation

Use this on a local device that cannot run PostgreSQL/MinIO/Docker:

```bash
cd ~/spark-api
bash scripts/karyra-pass52-server-runtime-hardening-audit.sh
cargo fmt --check
cargo check
cargo build
```

## Server validation flow

On the server:

```bash
cd ~/spark-api
cp config/env.server.example .env.server
```

Edit `.env.server` and replace all placeholder values.

Then run:

```bash
bash scripts/karyra-server-env-audit.sh .env.server
```

Bring runtime up using the server compose file:

```bash
docker compose --env-file .env.server -f infra/docker-compose.server.example.yml up -d --build
```

Apply migrations:

```bash
source .env.server
bash scripts/karyra-server-migration-check.sh "$DATABASE_URL"
```

Run API smoke tests:

```bash
bash scripts/karyra-server-api-smoke.sh http://127.0.0.1:8787
bash scripts/karyra-server-auth-proof-passport-smoke.sh http://127.0.0.1:8787
```

## Important boundaries

This pass does not add:

- Starknet wallet connection
- Cairo contracts
- Starknet Sepolia/Mainnet deploy
- NFT minting
- Public verifier
- Real S3 SigV4 signed upload URL

Those remain future milestone/grant scope.
