use std::path::Path;

use anyhow::Result;
use image::{ImageBuffer, ImageFormat, Rgba};

mod basic_decode;
mod gpu_resize;
mod missing_codec;
mod resize;
mod wic;
mod winrt;

pub use basic_decode::decode_jpeg_scaled;
pub use gpu_resize::{
    gpu_resize_enabled_value, gpu_resize_should_use, prewarm_gpu_resize_backend,
    resize_rgba8_srgb_gpu,
};
pub use missing_codec::{clear_missing_codec_kind, missing_codec_kinds};
pub use resize::{
    gamma_correct_resize_enabled_value, resize_linear_rgba8_exact, resize_linear_rgba8_fit,
    resize_rgba8_srgb_gamma_correct_into, ResizeGammaInto,
};

use basic_decode::{apply_icc_profile, decode_image_any, decode_png, guess_format};
use missing_codec::register_missing_codec;

pub type RgbaImage = ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum MissingCodecKind {
    Heif,
    Avif,
    Raw,
    Jpeg2000,
    JpegXr,
    Generic,
}

pub struct DecodedRgba {
    pub rgba: RgbaImage,
    // Contract: width/height always describe the returned rgba buffer size.
    // For scaled/thumbnail decode paths these are output dimensions, not source dimensions.
    pub width: u32,
    pub height: u32,
}

/// Probe image dimensions using built-in decoders first, then WIC/WinRT if available.
pub fn image_dimensions_any(path: &Path) -> Option<(u32, u32)> {
    if let Ok((w, h)) = image::image_dimensions(path) {
        return Some((w, h));
    }
    if let Ok(Some((w, h))) = wic::dimensions(path) {
        return Some((w, h));
    }
    None
}

pub fn image_format_any(path: &Path) -> Option<ImageFormat> {
    guess_format(path)
}

pub fn decode_rgba8_srgb(path: &Path) -> Result<DecodedRgba> {
    if let Ok(Some(decoded)) = decode_wic_full(path) {
        return Ok(decoded);
    }

    let format = guess_format(path);
    if let Some(ImageFormat::Jpeg) = format {
        return decode_jpeg_scaled(path, u32::MAX, u32::MAX);
    }

    let (img, icc) = match format {
        Some(ImageFormat::Png) => decode_png(path)?,
        _ => {
            let img = decode_image_any(path)?;
            (img, None)
        }
    };

    let mut rgba = img.to_rgba8();
    if let Some(profile) = icc {
        if let Err(err) = apply_icc_profile(&mut rgba, &profile) {
            log::warn!("ICC: failed to apply profile for {:?}: {err:?}", path);
        }
    }

    let (width, height) = rgba.dimensions();
    Ok(DecodedRgba {
        rgba,
        width,
        height,
    })
}

pub fn decode_rgba8_srgb_thumbnail(path: &Path, max_dim: u32) -> Result<DecodedRgba> {
    if let Ok(Some(decoded)) = wic::decode_thumbnail(path, max_dim) {
        return Ok(decoded);
    }

    let format = guess_format(path);
    if let Some(ImageFormat::Jpeg) = format {
        if let Ok(decoded) = decode_jpeg_scaled(path, max_dim, max_dim) {
            return Ok(decoded);
        }
    }

    decode_rgba8_srgb(path)
}

pub fn decode_wic_full(path: &Path) -> Result<Option<DecodedRgba>> {
    wic::decode_full(path)
}

pub fn probe_wic_codec_once(path: &Path) {
    wic::probe_codec_once(path);
}

pub fn decode_wic_scaled(path: &Path, width: u32, height: u32) -> Result<Option<DecodedRgba>> {
    wic::decode_scaled(path, width, height)
}

fn clear_missing_codec_for_path(path: &Path) {
    let Some(ext) = path.extension().map(|e| e.to_string_lossy().to_lowercase()) else {
        return;
    };
    if let Some(kind) = wic_codec_kind(ext.as_str()) {
        clear_missing_codec_kind(kind);
    }
}

fn register_missing_codec_for_path(path: &Path) {
    let Some(ext) = path.extension().map(|e| e.to_string_lossy().to_lowercase()) else {
        return;
    };
    if let Some(kind) = wic_codec_kind(ext.as_str()) {
        register_missing_codec(kind);
    }
}

#[cfg(target_os = "windows")]
fn wic_codec_kind(ext: &str) -> Option<MissingCodecKind> {
    use crate::core::formats::WicCodecFamily;

    Some(match crate::core::formats::wic_codec_family_for_ext(ext) {
        Some(WicCodecFamily::Heif) => MissingCodecKind::Heif,
        Some(WicCodecFamily::Avif) => MissingCodecKind::Avif,
        Some(WicCodecFamily::Raw) => MissingCodecKind::Raw,
        Some(WicCodecFamily::Jpeg2000) => MissingCodecKind::Jpeg2000,
        Some(WicCodecFamily::JpegXr) => MissingCodecKind::JpegXr,
        None => MissingCodecKind::Generic,
    })
}

#[cfg(not(target_os = "windows"))]
fn wic_codec_kind(_ext: &str) -> Option<MissingCodecKind> {
    None
}
