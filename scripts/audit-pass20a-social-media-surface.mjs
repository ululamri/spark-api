#!/usr/bin/env node
import { readFileSync } from 'node:fs';

const social = readFileSync('src/social/mod.rs', 'utf8');
const media = readFileSync('src/media/mod.rs', 'utf8');
const optimizer = readFileSync('src/media_optimizer.rs', 'utf8');

const required = [
  [social, 'require_current_user(&state, &headers).await?'],
  [social, 'moderation::enforce_rate_limit(&state, user.id, "social_post_create").await?'],
  [social, 'moderation::enforce_rate_limit(&state, user.id, "social_comment_create").await?'],
  [social, 'moderation::enforce_rate_limit(&state, user.id, "social_report_create").await?'],
  [social, 'moderation::evaluate_text(&body)'],
  [social, 'moderation::record_content_decision('],
  [social, 'attach_media_assets(&state, user.id, "social_post", post_id, media_asset_ids).await?'],
  [social, 'attach_media_assets('],
  [social, 'optimized_public_image_urls'],
  [media, 'validate_upload_request(&payload)?'],
  [media, 'MAX_UPLOAD_BYTES'],
  [media, 'normalize_mime_type(&payload.mime_type)?'],
  [media, 'where id = $1 and status = \'uploaded\' and visibility = \'public\''],
  [optimizer, 'imgproxy'],
  [optimizer, 'optimized_public_image_urls']
];

const blockers = [];
for (const [text, item] of required) {
  if (!text.includes(item)) blockers.push(`Missing ${item}`);
}

if (!media.includes('storage_access_url(&state, "HEAD"') && !media.includes('storage_access_url(state, "HEAD"')) {
  blockers.push('complete_upload does not verify uploaded object existence with a server-side HEAD request before marking asset uploaded');
}

if (media.includes('checksum = coalesce($3, checksum)')) {
  blockers.push('complete_upload still allows client payload to mutate checksum during completion');
}

if (media.includes('size_bytes = coalesce($4, size_bytes)')) {
  blockers.push('complete_upload still allows client payload to mutate size_bytes during completion');
}

console.log('PASS 20A social/media surface audit');
if (blockers.length) {
  for (const blocker of blockers) console.error(`- ${blocker}`);
  process.exit(1);
}
console.log('No PASS 20A social/media surface blockers found.');
