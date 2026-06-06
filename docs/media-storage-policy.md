# Media Storage Policy

Spark needs media for lessons, community posts, event pages, profile avatars, Passport assets, and proof evidence bundles.

## Storage decision

Use an S3-compatible storage interface from day one.

Default development:

```txt
MinIO
```

Small production:

```txt
Garage or MinIO self-hosted
```

Future scale:

```txt
Cloudflare R2 / AWS S3 / Backblaze / any S3-compatible provider
```

## Upload flow

```txt
SvelteKit frontend
-> Rust/Axum upload intent
-> backend validates auth, MIME type, size, purpose
-> backend returns signed upload URL
-> browser uploads directly to S3-compatible storage
-> backend confirms and stores metadata in PostgreSQL
```

Pass 45 clean only provides the route boundary and placeholder upload intent. Real signed URL support comes later.

## Initial restrictions

- No long video self-hosting before grant.
- Limit user-uploaded social media.
- Allow small images, avatars, lesson illustrations, event assets, PDF notes, badge assets, and private evidence JSON.
- Keep sensitive evidence private by default.
