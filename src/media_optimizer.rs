use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::config::AppConfig;

#[derive(Debug, Clone, Serialize)]
pub struct OptimizedMediaUrls {
    pub avatar_64: String,
    pub avatar_128: String,
    pub feed_480: String,
    pub feed_720: String,
    pub detail_1080: String,
    pub detail_1440: String,
    pub original: String,
}

#[derive(Debug, Clone, Copy)]
pub enum MediaFit {
    Fit,
    Fill,
}

pub fn optimized_public_image_urls(
    config: &AppConfig,
    public_url: &str,
    mime_type: &str,
) -> Option<OptimizedMediaUrls> {
    if !config.media_optimizer_enabled || !mime_type.starts_with("image/") {
        return None;
    }

    let source_url = absolute_source_url(config, public_url)?;
    let original = source_url.clone();

    Some(OptimizedMediaUrls {
        avatar_64: build_imgproxy_url(config, &source_url, 64, 64, MediaFit::Fill)?,
        avatar_128: build_imgproxy_url(config, &source_url, 128, 128, MediaFit::Fill)?,
        feed_480: build_imgproxy_url(config, &source_url, 480, 480, MediaFit::Fit)?,
        feed_720: build_imgproxy_url(config, &source_url, 720, 720, MediaFit::Fit)?,
        detail_1080: build_imgproxy_url(config, &source_url, 1080, 1080, MediaFit::Fit)?,
        detail_1440: build_imgproxy_url(config, &source_url, 1440, 1440, MediaFit::Fit)?,
        original,
    })
}

pub fn build_imgproxy_url(
    config: &AppConfig,
    source_url: &str,
    width: u32,
    height: u32,
    fit: MediaFit,
) -> Option<String> {
    let resize = match fit {
        MediaFit::Fit => "fit",
        MediaFit::Fill => "fill",
    };
    let path = format!(
        "/rs:{resize}:{width}:{height}/q:82/plain/{}@webp",
        source_url.trim()
    );
    let signature = imgproxy_signature(config, &path)?;
    Some(format!(
        "{}{}{}",
        config.imgproxy_public_base_url.trim_end_matches('/'),
        signature,
        path
    ))
}

fn absolute_source_url(config: &AppConfig, public_url: &str) -> Option<String> {
    let value = public_url.trim();
    if value.starts_with("https://") || value.starts_with("http://") {
        return Some(value.to_string());
    }
    if !value.starts_with('/') {
        return None;
    }
    Some(format!(
        "{}{}",
        config.imgproxy_source_base_url.trim_end_matches('/'),
        value
    ))
}

fn imgproxy_signature(config: &AppConfig, path: &str) -> Option<String> {
    let key = decode_hex(config.imgproxy_key_hex.as_deref()?)?;
    let salt = decode_hex(config.imgproxy_salt_hex.as_deref()?)?;
    let mut data = Vec::with_capacity(salt.len() + path.len());
    data.extend_from_slice(&salt);
    data.extend_from_slice(path.as_bytes());
    Some(format!("/{}", base64_url_no_pad(&hmac_sha256(&key, &data))))
}

fn decode_hex(input: &str) -> Option<Vec<u8>> {
    let value = input.trim();
    if value.len() % 2 != 0 || value.is_empty() {
        return None;
    }
    let mut output = Vec::with_capacity(value.len() / 2);
    for pair in value.as_bytes().chunks_exact(2) {
        let high = hex_value(pair[0])?;
        let low = hex_value(pair[1])?;
        output.push((high << 4) | low);
    }
    Some(output)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    const BLOCK_SIZE: usize = 64;
    let mut normalized_key = [0u8; BLOCK_SIZE];

    if key.len() > BLOCK_SIZE {
        let hashed = Sha256::digest(key);
        normalized_key[..32].copy_from_slice(&hashed);
    } else {
        normalized_key[..key.len()].copy_from_slice(key);
    }

    let mut outer_key_pad = [0x5cu8; BLOCK_SIZE];
    let mut inner_key_pad = [0x36u8; BLOCK_SIZE];
    for index in 0..BLOCK_SIZE {
        outer_key_pad[index] ^= normalized_key[index];
        inner_key_pad[index] ^= normalized_key[index];
    }

    let mut inner = Sha256::new();
    inner.update(inner_key_pad);
    inner.update(data);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(outer_key_pad);
    outer.update(inner_hash);
    let output = outer.finalize();

    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&output);
    bytes
}

fn base64_url_no_pad(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut output = String::with_capacity((bytes.len() * 4).div_ceil(3));
    let mut index = 0;

    while index + 3 <= bytes.len() {
        let chunk = ((bytes[index] as u32) << 16)
            | ((bytes[index + 1] as u32) << 8)
            | bytes[index + 2] as u32;
        output.push(TABLE[((chunk >> 18) & 0x3f) as usize] as char);
        output.push(TABLE[((chunk >> 12) & 0x3f) as usize] as char);
        output.push(TABLE[((chunk >> 6) & 0x3f) as usize] as char);
        output.push(TABLE[(chunk & 0x3f) as usize] as char);
        index += 3;
    }

    match bytes.len() - index {
        1 => {
            let chunk = (bytes[index] as u32) << 16;
            output.push(TABLE[((chunk >> 18) & 0x3f) as usize] as char);
            output.push(TABLE[((chunk >> 12) & 0x3f) as usize] as char);
        }
        2 => {
            let chunk = ((bytes[index] as u32) << 16) | ((bytes[index + 1] as u32) << 8);
            output.push(TABLE[((chunk >> 18) & 0x3f) as usize] as char);
            output.push(TABLE[((chunk >> 12) & 0x3f) as usize] as char);
            output.push(TABLE[((chunk >> 6) & 0x3f) as usize] as char);
        }
        _ => {}
    }

    output
}
