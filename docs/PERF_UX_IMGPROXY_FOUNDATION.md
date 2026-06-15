# PERF-UX-03 imgproxy Foundation

**Priority:** UI/UX performance before Admin UI and Starknet integration.

This pass prepares Spark for optimized public media delivery without changing the stable upload flow yet.

## Goal

Uploaded public images should eventually be delivered as responsive optimized variants instead of loading original-size objects in feed/profile/community surfaces.

Target variants:

```text
avatar_64
avatar_128
feed_480
feed_720
detail_1080
detail_1440
original
```

## Backend configuration

New `spark-api` environment variables:

```text
SPARK_MEDIA_OPTIMIZER_ENABLED=false
IMGPROXY_PUBLIC_BASE_URL=/media/optimized
IMGPROXY_SOURCE_BASE_URL=https://spark.user.cloudjkt01.com
IMGPROXY_KEY_HEX=
IMGPROXY_SALT_HEX=
```

Keep `SPARK_MEDIA_OPTIMIZER_ENABLED=false` until imgproxy is installed, signed URLs are verified, and Caddy `/media/optimized/*` is enabled.

## Added backend files

```text
src/media_optimizer.rs
```

This module can generate signed imgproxy URLs for public image media. It respects:

```text
public image only
optimizer enabled flag
hex key and salt
avatar/feed/detail variant sizes
webp output
```

The module is registered in `src/main.rs`, but existing upload and redirect behavior remains stable.

## Service files

```text
deploy/imgproxy.env.example
deploy/karyra-imgproxy.service.example
```

These are examples only. Live values should be stored under `/etc/karyra/imgproxy.env` or another secure server-only path.

## Caddy integration

The Spark Caddy example already contains a future block:

```caddy
# handle /media/optimized/* {
# 	header Cache-Control "public, max-age=2592000, stale-while-revalidate=86400"
# 	reverse_proxy 127.0.0.1:8088
# }
```

Do not enable this block until the imgproxy service is running and smoke-tested.

## Safe rollout order

```text
1. Install imgproxy on server.
2. Generate IMGPROXY_KEY_HEX and IMGPROXY_SALT_HEX.
3. Start imgproxy locally.
4. Test a signed URL from backend helper or a known static image.
5. Enable Caddy /media/optimized/* route.
6. Set SPARK_MEDIA_OPTIMIZER_ENABLED=true in spark-api env.
7. Restart spark-api.
8. Update frontend media components to consume optimized variants.
```

## Guardrails

```text
Do not optimize private media.
Do not expose unsigned imgproxy routes in production.
Do not replace original public URLs until optimized variants are visible and smoke-tested.
Do not load original images in feed after optimized variants are wired.
```

## Next pass

```text
PERF-UX-04 optimized media component and avatar/feed integration
```
