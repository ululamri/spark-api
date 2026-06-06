# Pass 45 Clean — Spark Backend Architecture

Spark backend is a separate repository/workspace from the SvelteKit frontend.

```txt
~/spark      -> SvelteKit frontend
~/spark-api  -> Rust/Axum backend
```

## Locked stack

- Frontend: SvelteKit
- Backend: Rust/Axum
- Database: PostgreSQL
- DB layer: SQLx
- Storage: S3-compatible object storage
- Local object storage: MinIO
- Small production object storage: Garage or MinIO self-hosted
- Future scale: Cloudflare R2/S3-compatible provider if needed

## Boundary

The backend owns production data and proof records:

- auth and sessions
- users and profiles
- Core/Lab exam attempts
- proof events
- Passport credentials
- media metadata
- community/social/Hub data in later passes

The frontend remains a public user app and consumes backend APIs through a thin API client.

## Do not overbuild before grant

Do not implement live Starknet contract integration, NFT minting, public verifier, or KYC in this phase. Those remain grant milestones.
