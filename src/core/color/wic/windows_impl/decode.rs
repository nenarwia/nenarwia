use std::path::Path;

use anyhow::Result;
use image::ImageBuffer;

use crate::core::color::DecodedRgba;

use super::errors::log_decode_error;
use super::icc::{apply_icc_if_any, extract_icc};
use super::interop::{create_decoder, create_factory};

struct OpenedFrame {
    factory: windows::Win32::Graphics::Imaging::IWICImagingFactory,
    frame: windows::Win32::Graphics::Imaging::IWICBitmapFrameDecode,
    src_w: u32,
    src_h: u32,
    icc: Option<Vec<u8>>,
}

fn open_frame_for_scaled_decode(path: &Path) -> Option<OpenedFrame> {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let Some(factory) = create_factory(path) else {
            return None;
        };
        let decoder = match create_decoder(&factory, path) {
            Ok(d) => d,
            Err(_) => return None,
        };
        let frame = match decoder.GetFrame(0) {
            Ok(f) => f,
            Err(err) => {
                log_decode_error(path, &err);
                return None;
            }
        };

        let mut src_w: u32 = 0;
        let mut src_h: u32 = 0;
        if let Err(err) = frame.GetSize(&mut src_w, &mut src_h) {
            log_decode_error(path, &err);
            return None;
        }
        if src_w == 0 || src_h == 0 {
            return None;
        }

        let icc = extract_icc(&frame);
        Some(OpenedFrame {
            factory,
            frame,
            src_w,
            src_h,
            icc,
        })
    }
}

fn decode_resized_from_opened(
    path: &Path,
    opened: OpenedFrame,
    out_w: u32,
    out_h: u32,
) -> Option<DecodedRgba> {
    use windows::Win32::Graphics::Imaging::{
        GUID_WICPixelFormat32bppBGRA, IWICBitmapScaler, IWICFormatConverter,
        WICBitmapDitherTypeNone, WICBitmapInterpolationModeFant, WICBitmapPaletteTypeCustom,
    };

    if out_w == 0 || out_h == 0 {
        return None;
    }

    let OpenedFrame {
        factory,
        frame,
        icc,
        ..
    } = opened;

    unsafe {
        let scaler: IWICBitmapScaler = match factory.CreateBitmapScaler() {
            Ok(s) => s,
            Err(err) => {
                log_decode_error(path, &err);
                return None;
            }
        };
        if let Err(err) = scaler.Initialize(&frame, out_w, out_h, WICBitmapInterpolationModeFant) {
            log_decode_error(path, &err);
            return None;
        }

        let converter: IWICFormatConverter = match factory.CreateFormatConverter() {
            Ok(c) => c,
            Err(err) => {
                log_decode_error(path, &err);
                return None;
            }
        };
        if let Err(err) = converter.Initialize(
            &scaler,
            &GUID_WICPixelFormat32bppBGRA,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeCustom,
        ) {
            log_decode_error(path, &err);
            return None;
        }

        let stride = out_w.saturating_mul(4);
        let buf_len = stride.saturating_mul(out_h) as usize;
        if buf_len == 0 {
            return None;
        }
        let mut buf = vec![0u8; buf_len];
        if let Err(err) = converter.CopyPixels(std::ptr::null(), stride, &mut buf) {
            log_decode_error(path, &err);
            return None;
        }

        for px in buf.chunks_mut(4) {
            px.swap(0, 2);
        }

        let Some(mut img) = ImageBuffer::from_raw(out_w, out_h, buf) else {
            return None;
        };

        apply_icc_if_any(path, &mut img, icc);
        super::super::super::clear_missing_codec_for_path(path);
        Some(DecodedRgba {
            rgba: img,
            width: out_w,
            height: out_h,
        })
    }
}

fn fit_within_max_dim(src_w: u32, src_h: u32, max_dim: u32) -> (u32, u32) {
    let scale = (max_dim as f32 / src_w as f32)
        .min(max_dim as f32 / src_h as f32)
        .min(1.0);
    let out_w = (src_w as f32 * scale).round().max(1.0) as u32;
    let out_h = (src_h as f32 * scale).round().max(1.0) as u32;
    (out_w, out_h)
}

pub(super) fn dimensions_inner(path: &Path) -> Result<Option<(u32, u32)>> {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let Some(factory) = create_factory(path) else {
            return Ok(None);
        };
        let decoder = match create_decoder(&factory, path) {
            Ok(d) => d,
            Err(_) => return Ok(None),
        };
        let frame = match decoder.GetFrame(0) {
            Ok(f) => f,
            Err(err) => {
                log_decode_error(path, &err);
                return Ok(None);
            }
        };

        let mut w: u32 = 0;
        let mut h: u32 = 0;
        if let Err(err) = frame.GetSize(&mut w, &mut h) {
            log_decode_error(path, &err);
            return Ok(None);
        }
        if w == 0 || h == 0 {
            return Ok(None);
        }
        Ok(Some((w, h)))
    }
}

pub(super) fn decode_full_inner(path: &Path) -> Result<Option<DecodedRgba>> {
    use windows::Win32::Graphics::Imaging::{
        GUID_WICPixelFormat32bppBGRA, IWICFormatConverter, WICBitmapDitherTypeNone,
        WICBitmapPaletteTypeCustom,
    };
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let Some(factory) = create_factory(path) else {
            return Ok(None);
        };
        let decoder = match create_decoder(&factory, path) {
            Ok(d) => d,
            Err(_) => return Ok(None),
        };
        let frame = decoder.GetFrame(0).map_err(|err| {
            log_decode_error(path, &err);
            anyhow::anyhow!("wic frame failed: {err:?}")
        })?;

        let mut w: u32 = 0;
        let mut h: u32 = 0;
        frame.GetSize(&mut w, &mut h).map_err(|err| {
            log_decode_error(path, &err);
            anyhow::anyhow!("wic size failed: {err:?}")
        })?;
        if w == 0 || h == 0 {
            return Ok(None);
        }

        let icc = extract_icc(&frame);

        let converter: IWICFormatConverter = factory.CreateFormatConverter().map_err(|err| {
            log_decode_error(path, &err);
            anyhow::anyhow!("wic converter failed: {err:?}")
        })?;
        converter
            .Initialize(
                &frame,
                &GUID_WICPixelFormat32bppBGRA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeCustom,
            )
            .map_err(|err| {
                log_decode_error(path, &err);
                anyhow::anyhow!("wic convert init failed: {err:?}")
            })?;

        let stride = w.saturating_mul(4);
        let buf_len = stride.saturating_mul(h) as usize;
        if buf_len == 0 {
            return Ok(None);
        }
        let mut buf = vec![0u8; buf_len];
        converter
            .CopyPixels(std::ptr::null(), stride, &mut buf)
            .map_err(|err| {
                log_decode_error(path, &err);
                anyhow::anyhow!("wic copy pixels failed: {err:?}")
            })?;

        for px in buf.chunks_mut(4) {
            px.swap(0, 2);
        }

        let Some(mut img) = ImageBuffer::from_raw(w, h, buf) else {
            return Ok(None);
        };

        apply_icc_if_any(path, &mut img, icc);
        super::super::super::clear_missing_codec_for_path(path);
        Ok(Some(DecodedRgba {
            rgba: img,
            width: w,
            height: h,
        }))
    }
}

pub(super) fn decode_thumbnail_inner(path: &Path, max_dim: u32) -> Result<Option<DecodedRgba>> {
    if max_dim == 0 {
        return Ok(None);
    }

    let Some(opened) = open_frame_for_scaled_decode(path) else {
        return Ok(None);
    };
    let (out_w, out_h) = fit_within_max_dim(opened.src_w, opened.src_h, max_dim.max(1));
    Ok(decode_resized_from_opened(path, opened, out_w, out_h))
}

pub(super) fn decode_scaled_inner(
    path: &Path,
    width: u32,
    height: u32,
) -> Result<Option<DecodedRgba>> {
    if width == 0 || height == 0 {
        return Ok(None);
    }

    let Some(opened) = open_frame_for_scaled_decode(path) else {
        return Ok(None);
    };
    Ok(decode_resized_from_opened(
        path,
        opened,
        width.max(1),
        height.max(1),
    ))
}
