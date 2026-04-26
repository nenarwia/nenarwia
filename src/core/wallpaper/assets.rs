use std::fs;
use std::io;
use std::path::Path;

use anyhow::{Context, Result};
use image::codecs::jpeg::JpegEncoder;
use image::ImageFormat;

use crate::core::color;

use super::{WALLPAPER_JPEG_QUALITY, WALLPAPER_STORAGE_MAX_DIM};

pub fn persist_wallpaper_source(source_path: &Path, stored_path: &Path) -> Result<()> {
    if should_copy_jpeg_as_is(source_path) {
        if let Some(parent) = stored_path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create '{}'", parent.display()))?;
        }
        fs::copy(source_path, stored_path).with_context(|| {
            format!(
                "copy wallpaper jpeg '{}' -> '{}'",
                source_path.display(),
                stored_path.display()
            )
        })?;
        return Ok(());
    }

    let decoded = color::decode_rgba8_srgb(source_path)
        .with_context(|| format!("decode wallpaper source '{}'", source_path.display()))?;
    let normalized = normalize_wallpaper_source(decoded);
    let (width, height) = normalized.dimensions();
    save_rgba_as_jpeg(
        stored_path,
        normalized.as_raw(),
        width,
        height,
        WALLPAPER_JPEG_QUALITY,
    )
    .with_context(|| format!("save wallpaper jpeg '{}'", stored_path.display()))?;
    Ok(())
}

pub fn persist_wallpaper_jpeg_bytes(source_bytes: &[u8], stored_path: &Path) -> Result<()> {
    let image = image::load_from_memory_with_format(source_bytes, ImageFormat::Jpeg)
        .context("decode bundled wallpaper jpeg")?;
    let mut rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    if width == 0 || height == 0 {
        anyhow::bail!("bundled wallpaper jpeg has invalid dimensions");
    }

    if width.max(height) <= WALLPAPER_STORAGE_MAX_DIM {
        if let Some(parent) = stored_path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create '{}'", parent.display()))?;
        }
        fs::write(stored_path, source_bytes)
            .with_context(|| format!("write bundled wallpaper jpeg '{}'", stored_path.display()))?;
        return Ok(());
    }

    rgba =
        color::resize_linear_rgba8_fit(&rgba, WALLPAPER_STORAGE_MAX_DIM, WALLPAPER_STORAGE_MAX_DIM);
    let (width, height) = rgba.dimensions();
    save_rgba_as_jpeg(
        stored_path,
        rgba.as_raw(),
        width,
        height,
        WALLPAPER_JPEG_QUALITY,
    )
    .with_context(|| format!("save bundled wallpaper jpeg '{}'", stored_path.display()))?;
    Ok(())
}

fn should_copy_jpeg_as_is(source_path: &Path) -> bool {
    match color::image_format_any(source_path) {
        Some(ImageFormat::Jpeg) => color::image_dimensions_any(source_path)
            .map(|(width, height)| width.max(height) <= WALLPAPER_STORAGE_MAX_DIM)
            .unwrap_or(false),
        _ => false,
    }
}

fn normalize_wallpaper_source(decoded: color::DecodedRgba) -> color::RgbaImage {
    if decoded.width.max(decoded.height) <= WALLPAPER_STORAGE_MAX_DIM {
        return decoded.rgba;
    }
    color::resize_linear_rgba8_fit(
        &decoded.rgba,
        WALLPAPER_STORAGE_MAX_DIM,
        WALLPAPER_STORAGE_MAX_DIM,
    )
}

pub(super) fn save_rgba_as_jpeg(
    path: &Path,
    pixels: &[u8],
    width: u32,
    height: u32,
    quality: u8,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create '{}'", parent.display()))?;
    }
    let mut rgb = Vec::with_capacity((width as usize) * (height as usize) * 3);
    for chunk in pixels.chunks_exact(4) {
        let alpha = chunk[3] as u16;
        rgb.push(((chunk[0] as u16 * alpha + 127) / 255) as u8);
        rgb.push(((chunk[1] as u16 * alpha + 127) / 255) as u8);
        rgb.push(((chunk[2] as u16 * alpha + 127) / 255) as u8);
    }

    let file = fs::File::create(path).with_context(|| format!("create '{}'", path.display()))?;
    let writer = io::BufWriter::new(file);
    let mut encoder = JpegEncoder::new_with_quality(writer, quality);
    encoder
        .encode(&rgb, width, height, image::ExtendedColorType::Rgb8)
        .with_context(|| format!("encode jpeg '{}'", path.display()))?;
    Ok(())
}
