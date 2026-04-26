use std::fs;
use std::path::PathBuf;

use image::{ImageBuffer, Rgba};

use crate::core::loader::disk_cache as dc;
use crate::core::loader::mem_cache as mc;
use crate::core::loader::types::{ImagePayload, LoadedImage};
use crate::core::loader::ThumbDecodeMode;
use crate::core::{color, metrics};

use super::{acquire_decode_guard, allow_runtime_decode, make_thumb, resolve_dims};

fn fast_preview_decode_cap_px() -> u32 {
    use std::sync::OnceLock;
    static CAP: OnceLock<u32> = OnceLock::new();
    *CAP.get_or_init(|| {
        std::env::var("CANVAS_FAST_PREVIEW_DECODE_CAP_PX")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .map(|v| v.clamp(64, 1024))
            .unwrap_or(192)
    })
}

fn medium_preview_decode_cap_px() -> u32 {
    use std::sync::OnceLock;
    static CAP: OnceLock<u32> = OnceLock::new();
    *CAP.get_or_init(|| {
        std::env::var("CANVAS_MEDIUM_PREVIEW_DECODE_CAP_PX")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .map(|v| v.clamp(96, 2048))
            .unwrap_or(384)
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn process_thumbnail(
    path: &PathBuf,
    id: u64,
    asset_key: u64,
    size: u16,
    decode_mode: ThumbDecodeMode,
    orig_w_hint: u32,
    orig_h_hint: u32,
    epoch: u64,
) -> LoadedImage {
    let mut orig_w = orig_w_hint;
    let mut orig_h = orig_h_hint;

    if !mc::is_ram_media_slot_asset(asset_key) {
        resolve_dims(path, &mut orig_w, &mut orig_h);
        return make_thumb(
            id,
            asset_key,
            epoch,
            size,
            ImagePayload::Rgba8(Vec::new()),
            true,
            orig_w,
            orig_h,
        );
    }

    if let Some(bytes) = mc::get_thumb(asset_key, size) {
        resolve_dims(path, &mut orig_w, &mut orig_h);
        return make_thumb(
            id,
            asset_key,
            epoch,
            size,
            ImagePayload::Rgba8(bytes),
            false,
            orig_w,
            orig_h,
        );
    }

    if let Ok(bytes) = dc::read_thumb_rgba_lz4(asset_key, size) {
        resolve_dims(path, &mut orig_w, &mut orig_h);
        mc::put_thumb(asset_key, size, bytes.clone());
        return make_thumb(
            id,
            asset_key,
            epoch,
            size,
            ImagePayload::Rgba8(bytes),
            false,
            orig_w,
            orig_h,
        );
    }

    if !allow_runtime_decode() {
        resolve_dims(path, &mut orig_w, &mut orig_h);
        return make_thumb(
            id,
            asset_key,
            epoch,
            size,
            ImagePayload::Rgba8(Vec::new()),
            true,
            orig_w,
            orig_h,
        );
    }

    let _decode_guard = acquire_decode_guard();
    // Decode budget scales with motion profile:
    // draft (fast move), medium (slower move), full (stable/low speed).
    let thumb_req = match decode_mode {
        ThumbDecodeMode::Draft => {
            metrics::record_thumb_preview_draft_job();
            (size as u32).max(64).min(fast_preview_decode_cap_px())
        }
        ThumbDecodeMode::Medium => {
            metrics::record_thumb_preview_medium_job();
            (size as u32).max(96).min(medium_preview_decode_cap_px())
        }
        ThumbDecodeMode::Full => (size as u32).saturating_mul(2).max(64),
    };
    if let Ok(decoded) = color::decode_rgba8_srgb_thumbnail(path, thumb_req) {
        if let Ok(meta) = fs::metadata(path) {
            metrics::record_io_read(meta.len());
        }
        resolve_dims(path, &mut orig_w, &mut orig_h);
        if orig_w == 0 || orig_h == 0 {
            orig_w = decoded.width;
            orig_h = decoded.height;
        }

        let scaled = color::resize_linear_rgba8_fit(&decoded.rgba, size as u32, size as u32);

        let mut buffer = ImageBuffer::from_pixel(size as u32, size as u32, Rgba([0, 0, 0, 0]));

        let dx = (((size as i32) - scaled.width() as i32) / 2).max(0) as i64;
        let dy = (((size as i32) - scaled.height() as i32) / 2).max(0) as i64;
        image::imageops::overlay(&mut buffer, &scaled, dx, dy);

        let raw = buffer.into_raw();
        if matches!(decode_mode, ThumbDecodeMode::Full) {
            let _ = dc::write_thumb_rgba_lz4(asset_key, size, &raw);
        }
        mc::put_thumb(asset_key, size, raw.clone());

        return make_thumb(
            id,
            asset_key,
            epoch,
            size,
            ImagePayload::Rgba8(raw),
            false,
            orig_w,
            orig_h,
        );
    }

    resolve_dims(path, &mut orig_w, &mut orig_h);
    make_thumb(
        id,
        asset_key,
        epoch,
        size,
        ImagePayload::Rgba8(Vec::new()),
        true,
        orig_w,
        orig_h,
    )
}
