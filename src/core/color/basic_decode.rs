use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use image::codecs::png::PngDecoder;
use image::{DynamicImage, ImageBuffer, ImageDecoder, ImageFormat, ImageReader, Rgba};
use jpeg_decoder::{Decoder as JpegFastDecoder, PixelFormat as JpegFastPixelFormat};
use moxcms::{ColorProfile, Layout, TransformOptions};
use turbojpeg::{
    Decompressor as TurboDecompressor, Image as TurboImage, PixelFormat as TurboPixelFormat,
    ScalingFactor as TurboScalingFactor,
};

use super::{DecodedRgba, RgbaImage};
use crate::core::metrics;

pub fn decode_jpeg_scaled(path: &Path, req_w: u32, req_h: u32) -> Result<DecodedRgba> {
    if turbojpeg_enabled() {
        if let Ok(decoded) = decode_jpeg_scaled_turbo(path, req_w, req_h) {
            return Ok(decoded);
        }
        metrics::record_jpeg_turbo_fallback();
    }
    decode_jpeg_scaled_builtin(path, req_w, req_h)
}

fn turbojpeg_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| match std::env::var("CANVAS_TURBOJPEG") {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            !matches!(v.as_str(), "0" | "false" | "off" | "no")
        }
        Err(_) => true,
    })
}

fn decode_jpeg_scaled_turbo(path: &Path, req_w: u32, req_h: u32) -> Result<DecodedRgba> {
    let jpeg_data = std::fs::read(path).context("read jpeg bytes")?;
    let start = Instant::now();

    let mut decompressor = TurboDecompressor::new().context("create turbojpeg decompressor")?;
    let header = decompressor
        .read_header(&jpeg_data)
        .context("turbojpeg read header")?;
    let scaling = choose_turbo_scaling(header.width, header.height, req_w, req_h);
    if scaling != TurboScalingFactor::ONE {
        decompressor
            .set_scaling_factor(scaling)
            .context("turbojpeg set scaling factor")?;
    }
    let scaled_header = header.scaled(scaling);

    let out_w = scaled_header.width.max(1);
    let out_h = scaled_header.height.max(1);
    let out_len = out_w
        .checked_mul(out_h)
        .and_then(|px| px.checked_mul(4))
        .ok_or_else(|| anyhow!("turbojpeg output size overflow"))?;

    let mut image = TurboImage {
        pixels: vec![0u8; out_len],
        width: out_w,
        pitch: out_w.saturating_mul(4),
        height: out_h,
        format: TurboPixelFormat::RGBA,
    };

    decompressor
        .decompress(&jpeg_data, image.as_deref_mut())
        .context("turbojpeg decompress")?;

    let mut rgba = ImageBuffer::from_raw(out_w as u32, out_h as u32, image.pixels)
        .ok_or_else(|| anyhow!("turbojpeg returned invalid image buffer"))?;

    if let Some(icc) = read_jpeg_icc_profile_from_bytes(&jpeg_data) {
        if let Err(err) = apply_icc_profile(&mut rgba, &icc) {
            log::warn!("ICC: failed to apply profile for {:?}: {err:?}", path);
        }
    }

    metrics::record_jpeg_turbo_success(
        header.width,
        header.height,
        out_w,
        out_h,
        scaling != TurboScalingFactor::ONE,
    );
    metrics::record_jpeg_turbo_ms(start.elapsed().as_millis() as u64);

    Ok(DecodedRgba {
        rgba,
        width: out_w as u32,
        height: out_h as u32,
    })
}

fn choose_turbo_scaling(src_w: usize, src_h: usize, req_w: u32, req_h: u32) -> TurboScalingFactor {
    if req_w == u32::MAX || req_h == u32::MAX {
        return TurboScalingFactor::ONE;
    }

    let target_w = req_w.max(1).min(src_w as u32) as usize;
    let target_h = req_h.max(1).min(src_h as u32) as usize;
    let mut best: Option<(usize, TurboScalingFactor)> = None;

    for sf in TurboDecompressor::supported_scaling_factors() {
        let sw = sf.scale(src_w);
        let sh = sf.scale(src_h);
        if sw < target_w || sh < target_h {
            continue;
        }
        let area = sw.saturating_mul(sh);
        let replace = match best {
            None => true,
            Some((best_area, _)) => area < best_area,
        };
        if replace {
            best = Some((area, sf));
        }
    }

    best.map(|(_, sf)| sf).unwrap_or(TurboScalingFactor::ONE)
}

fn read_jpeg_icc_profile_from_bytes(jpeg_data: &[u8]) -> Option<Vec<u8>> {
    let cursor = std::io::Cursor::new(jpeg_data);
    let mut decoder = JpegFastDecoder::new(cursor);
    if decoder.read_info().is_err() {
        return None;
    }
    decoder.icc_profile()
}

fn decode_jpeg_scaled_builtin(path: &Path, req_w: u32, req_h: u32) -> Result<DecodedRgba> {
    let scaled_req = req_w != u32::MAX && req_h != u32::MAX;
    let file = File::open(path).context("open jpeg")?;
    let reader = BufReader::new(file);
    let mut decoder = JpegFastDecoder::new(reader);

    if scaled_req {
        let req_w = req_w.min(u16::MAX as u32) as u16;
        let req_h = req_h.min(u16::MAX as u32) as u16;
        let _ = decoder.scale(req_w.max(1), req_h.max(1));
    }
    let pixels = decoder.decode().context("jpeg decode")?;
    let info = decoder.info().context("jpeg info")?;
    let mut rgba = match info.pixel_format {
        JpegFastPixelFormat::RGB24 => {
            rgb24_to_rgba(&pixels, info.width as u32, info.height as u32)?
        }
        JpegFastPixelFormat::L8 => l8_to_rgba(&pixels, info.width as u32, info.height as u32)?,
        _ => {
            let img = image::open(path).context("decode jpeg fallback")?;
            img.to_rgba8()
        }
    };

    if let Some(icc) = decoder.icc_profile() {
        if let Err(err) = apply_icc_profile(&mut rgba, &icc) {
            log::warn!("ICC: failed to apply profile for {:?}: {err:?}", path);
        }
    }
    metrics::record_jpeg_builtin_used(scaled_req);

    Ok(DecodedRgba {
        rgba,
        width: info.width as u32,
        height: info.height as u32,
    })
}

pub(super) fn guess_format(path: &Path) -> Option<ImageFormat> {
    let file = File::open(path).ok()?;
    let reader = ImageReader::new(BufReader::new(file));
    let reader = reader.with_guessed_format().ok()?;
    reader
        .format()
        .or_else(|| ImageFormat::from_path(path).ok())
}

pub(super) fn decode_image_any(path: &Path) -> Result<DynamicImage> {
    let file = File::open(path).context("open image")?;
    let reader = ImageReader::new(BufReader::new(file))
        .with_guessed_format()
        .context("guess image format")?;
    reader.decode().context("decode image")
}

pub(super) fn decode_png(path: &Path) -> Result<(DynamicImage, Option<Vec<u8>>)> {
    let file = File::open(path).context("open png")?;
    let reader = BufReader::new(file);
    let mut decoder = PngDecoder::new(reader).context("png decoder")?;
    let icc = match decoder.icc_profile() {
        Ok(profile) => profile,
        Err(err) => {
            log::warn!("ICC: failed to read png profile for {:?}: {err:?}", path);
            None
        }
    };
    let img = DynamicImage::from_decoder(decoder).context("png decode")?;
    Ok((img, icc))
}

pub(super) fn apply_icc_profile(rgba: &mut RgbaImage, icc: &[u8]) -> Result<()> {
    if icc.is_empty() {
        return Ok(());
    }

    let src_profile = ColorProfile::new_from_slice(icc).context("parse icc profile")?;
    let dst_profile = ColorProfile::new_srgb();
    let transform = src_profile
        .create_transform_8bit(
            Layout::Rgb,
            &dst_profile,
            Layout::Rgb,
            TransformOptions::default(),
        )
        .context("icc transform")?;

    let (w, h) = rgba.dimensions();
    let row_rgb = (w as usize).saturating_mul(3);
    let row_rgba = (w as usize).saturating_mul(4);
    let mut src_row = vec![0u8; row_rgb];
    let mut dst_row = vec![0u8; row_rgb];
    let flat = rgba.as_flat_samples_mut();
    let samples = flat.samples;

    for y in 0..h as usize {
        let row = &samples[y * row_rgba..(y + 1) * row_rgba];
        for x in 0..w as usize {
            let i4 = x * 4;
            let i3 = x * 3;
            src_row[i3] = row[i4];
            src_row[i3 + 1] = row[i4 + 1];
            src_row[i3 + 2] = row[i4 + 2];
        }

        transform
            .transform(&src_row, &mut dst_row)
            .context("icc transform row")?;

        let row_mut = &mut samples[y * row_rgba..(y + 1) * row_rgba];
        for x in 0..w as usize {
            let i4 = x * 4;
            let i3 = x * 3;
            row_mut[i4] = dst_row[i3];
            row_mut[i4 + 1] = dst_row[i3 + 1];
            row_mut[i4 + 2] = dst_row[i3 + 2];
        }
    }

    Ok(())
}

fn rgb24_to_rgba(src: &[u8], w: u32, h: u32) -> Result<RgbaImage> {
    let px = (w as usize).saturating_mul(h as usize);
    if src.len() < px.saturating_mul(3) {
        return Ok(ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 0])));
    }
    let mut out = vec![0u8; px.saturating_mul(4)];
    for i in 0..px {
        let s = i * 3;
        let d = i * 4;
        out[d] = src[s];
        out[d + 1] = src[s + 1];
        out[d + 2] = src[s + 2];
        out[d + 3] = 255;
    }
    Ok(ImageBuffer::from_raw(w, h, out)
        .unwrap_or_else(|| ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 0]))))
}

fn l8_to_rgba(src: &[u8], w: u32, h: u32) -> Result<RgbaImage> {
    let px = (w as usize).saturating_mul(h as usize);
    if src.len() < px {
        return Ok(ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 0])));
    }
    let mut out = vec![0u8; px.saturating_mul(4)];
    for (i, v) in src.iter().enumerate().take(px) {
        let d = i * 4;
        out[d] = *v;
        out[d + 1] = *v;
        out[d + 2] = *v;
        out[d + 3] = 255;
    }
    Ok(ImageBuffer::from_raw(w, h, out)
        .unwrap_or_else(|| ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 0]))))
}
