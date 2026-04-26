use std::path::Path;

use anyhow::{Context, Result};

use crate::core::color;

use super::assets::save_rgba_as_jpeg;
use super::WALLPAPER_JPEG_QUALITY;

pub fn build_blurred_pixels(
    width: u32,
    height: u32,
    pixels: &[u8],
    blur_max_dim: u32,
) -> Result<(Vec<u8>, u32, u32)> {
    if width == 0 || height == 0 {
        anyhow::bail!("Invalid blur source dimensions.");
    }
    if pixels.len() < (width as usize) * (height as usize) * 4 {
        anyhow::bail!("Invalid blur source buffer.");
    }

    let largest = width.max(height).max(1);
    let (work_pixels, work_w, work_h) = if largest > blur_max_dim.max(1) {
        let scale = blur_max_dim as f32 / largest as f32;
        let target_w = ((width as f32 * scale).round() as u32).max(1);
        let target_h = ((height as f32 * scale).round() as u32).max(1);
        let resized = resample_bilinear(pixels, width, height, target_w, target_h);
        (resized, target_w, target_h)
    } else {
        (pixels.to_vec(), width, height)
    };

    let blurred = blur_11tap_separable(&work_pixels, work_w, work_h);
    Ok((blurred, work_w, work_h))
}

pub fn wallpaper_blur_max_dim_for_surface(surface_width: u32, surface_height: u32) -> u32 {
    (surface_width.max(surface_height) / 4)
        .clamp(320, 960)
        .max(1)
}

pub fn ensure_saved_wallpaper_preview_blur(
    source_path: &Path,
    blur_path: &Path,
    preview_blur_max_dim: u32,
) -> Result<()> {
    let preview_blur_max_dim = preview_blur_max_dim.max(1);
    if saved_wallpaper_preview_blur_matches(source_path, blur_path, preview_blur_max_dim) {
        return Ok(());
    }

    let decoded = color::decode_rgba8_srgb_thumbnail(source_path, preview_blur_max_dim)
        .with_context(|| {
            format!(
                "decode wallpaper preview blur source '{}'",
                source_path.display()
            )
        })?;
    let mut rgba = decoded.rgba;
    if decoded.width.max(decoded.height) > preview_blur_max_dim {
        rgba = color::resize_linear_rgba8_fit(&rgba, preview_blur_max_dim, preview_blur_max_dim);
    }
    let (source_w, source_h) = rgba.dimensions();
    let (blurred, blurred_w, blurred_h) =
        build_blurred_pixels(source_w, source_h, rgba.as_raw(), preview_blur_max_dim)
            .context("build wallpaper preview blur pixels")?;
    save_rgba_as_jpeg(
        blur_path,
        blurred.as_slice(),
        blurred_w,
        blurred_h,
        WALLPAPER_JPEG_QUALITY,
    )
    .with_context(|| format!("save wallpaper preview blur '{}'", blur_path.display()))?;
    Ok(())
}

fn saved_wallpaper_preview_blur_matches(
    source_path: &Path,
    blur_path: &Path,
    preview_blur_max_dim: u32,
) -> bool {
    if !blur_path.exists() {
        return false;
    }
    let Some((source_w, source_h)) = color::image_dimensions_any(source_path) else {
        return false;
    };
    let Some((blur_w, blur_h)) = color::image_dimensions_any(blur_path) else {
        return false;
    };
    let expected_max = source_w.max(source_h).min(preview_blur_max_dim.max(1));
    blur_w.max(blur_h) == expected_max
}

fn resample_bilinear(src: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Vec<u8> {
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return Vec::new();
    }
    if src.len() < (src_w as usize) * (src_h as usize) * 4 {
        return Vec::new();
    }

    let mut dst = vec![0u8; (dst_w as usize) * (dst_h as usize) * 4];
    let sx = src_w as f32 / dst_w as f32;
    let sy = src_h as f32 / dst_h as f32;

    for y in 0..dst_h {
        let src_y = (y as f32 + 0.5) * sy - 0.5;
        let y0 = src_y.floor() as i32;
        let fy = src_y - y0 as f32;
        let y0c = y0.clamp(0, src_h.saturating_sub(1) as i32) as u32;
        let y1c = (y0 + 1).clamp(0, src_h.saturating_sub(1) as i32) as u32;

        for x in 0..dst_w {
            let src_x = (x as f32 + 0.5) * sx - 0.5;
            let x0 = src_x.floor() as i32;
            let fx = src_x - x0 as f32;
            let x0c = x0.clamp(0, src_w.saturating_sub(1) as i32) as u32;
            let x1c = (x0 + 1).clamp(0, src_w.saturating_sub(1) as i32) as u32;

            let idx00 = ((y0c * src_w + x0c) * 4) as usize;
            let idx10 = ((y0c * src_w + x1c) * 4) as usize;
            let idx01 = ((y1c * src_w + x0c) * 4) as usize;
            let idx11 = ((y1c * src_w + x1c) * 4) as usize;

            let w00 = (1.0 - fx) * (1.0 - fy);
            let w10 = fx * (1.0 - fy);
            let w01 = (1.0 - fx) * fy;
            let w11 = fx * fy;

            let out_idx = ((y * dst_w + x) * 4) as usize;
            for c in 0..4 {
                let v = src[idx00 + c] as f32 * w00
                    + src[idx10 + c] as f32 * w10
                    + src[idx01 + c] as f32 * w01
                    + src[idx11 + c] as f32 * w11;
                dst[out_idx + c] = v.round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    dst
}

fn blur_11tap_separable(src: &[u8], width: u32, height: u32) -> Vec<u8> {
    const WEIGHTS: [f32; 6] = [0.1423, 0.1346, 0.1140, 0.0863, 0.0585, 0.0355];

    if width == 0 || height == 0 {
        return Vec::new();
    }
    if src.len() < (width as usize) * (height as usize) * 4 {
        return Vec::new();
    }

    let mut tmp = vec![0u8; src.len()];
    let mut dst = vec![0u8; src.len()];
    let w = width as i32;
    let h = height as i32;

    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0f32; 4];
            for k in -5..=5 {
                let sx = (x + k).clamp(0, w - 1) as u32;
                let sy = y as u32;
                let weight = WEIGHTS[k.unsigned_abs() as usize];
                let idx = ((sy * width + sx) * 4) as usize;
                for c in 0..4 {
                    acc[c] += src[idx + c] as f32 * weight;
                }
            }
            let out_idx = (((y as u32) * width + (x as u32)) * 4) as usize;
            for c in 0..4 {
                tmp[out_idx + c] = acc[c].round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0f32; 4];
            for k in -5..=5 {
                let sx = x as u32;
                let sy = (y + k).clamp(0, h - 1) as u32;
                let weight = WEIGHTS[k.unsigned_abs() as usize];
                let idx = ((sy * width + sx) * 4) as usize;
                for c in 0..4 {
                    acc[c] += tmp[idx + c] as f32 * weight;
                }
            }
            let out_idx = (((y as u32) * width + (x as u32)) * 4) as usize;
            for c in 0..4 {
                dst[out_idx + c] = acc[c].round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    dst
}
