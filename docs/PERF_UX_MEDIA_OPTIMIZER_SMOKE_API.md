# PERF-UX Media Optimizer Smoke API

This pass adds a small public smoke API for verifying imgproxy readiness before the social feed starts depending on optimized media variants.

## Routes

```text
GET /v1/media-optimizer/scope
GET /v1/media-optimizer/public/:asset_id/urls
```

## Scope response

`/v1/media-optimizer/scope` returns:

```text
enabled
public_base_url
source_base_url
key_configured
salt_configured
variants
```

This is safe to expose because it does not return secret values.

## Public asset URL response

`/v1/media-optimizer/public/:asset_id/urls` only works for public uploaded media assets that are allowed/restored by moderation.

When `SPARK_MEDIA_OPTIMIZER_ENABLED=false`, `optimized_urls` will be `null`.

When imgproxy is configured and `SPARK_MEDIA_OPTIMIZER_ENABLED=true`, the response can include:

```text
avatar_64
avatar_128
feed_480
feed_720
detail_1080
detail_1440
original
```

## Smoke tests

```bash
curl -i -sS https://spark.user.cloudjkt01.com/v1/media-optimizer/scope | head -80
```

After choosing a known public uploaded image asset ID:

```bash
curl -i -sS https://spark.user.cloudjkt01.com/v1/media-optimizer/public/<asset-id>/urls | head -120
```

## Guardrails

```text
Do not expose private media through this route.
Do not enable social feed dependency on optimized URLs until this smoke endpoint is verified.
Do not expose IMGPROXY_KEY_HEX or IMGPROXY_SALT_HEX in any API response.
```
