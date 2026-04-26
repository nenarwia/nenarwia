use std::sync::OnceLock;
use std::time::Instant;

use fast_image_resize as fir;
use image::imageops::FilterType;
use image::{ImageBuffer, Rgba};

use crate::core::metrics;

use super::RgbaImage;

pub fn resize_linear_rgba8_exact(src: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    if width == 0 || height == 0 {
        return ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 0]));
    }
    if (width, height) == src.dimensions() {
        return src.clone();
    }

    if gamma_correct_resize_enabled() {
        let (src_w, src_h) = src.dimensions();
        let filter = pick_gamma_filter(src_w, src_h, width, height);
        return resize_rgba8_srgb_gamma_correct_image(src, width, height, filter);
    }

    let linear = rgba8_to_linear_premultiplied(src);
    let resized = image::imageops::resize(&linear, width, height, FilterType::Triangle);
    linear_premultiplied_to_rgba8(&resized)
}

pub fn resize_linear_rgba8_fit(src: &RgbaImage, max_w: u32, max_h: u32) -> RgbaImage {
    if max_w == 0 || max_h == 0 {
        return ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 0]));
    }
    let (w, h) = src.dimensions();
    if w == 0 || h == 0 {
        return ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 0]));
    }

    let scale_w = max_w as f32 / w as f32;
    let scale_h = max_h as f32 / h as f32;
    let scale = scale_w.min(scale_h);
    let out_w = ((w as f32 * scale).round() as u32).max(1);
    let out_h = ((h as f32 * scale).round() as u32).max(1);
    resize_linear_rgba8_exact(src, out_w, out_h)
}

fn resize_rgba8_srgb_gamma_correct(
    src_rgba8: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
    filter: FilterType,
) -> Vec<u8> {
    let out_w = dst_w.max(1);
    let out_h = dst_h.max(1);
    let out_len = (out_w as usize)
        .saturating_mul(out_h as usize)
        .saturating_mul(4);
    let mut out = vec![0u8; out_len];
    let mut tmp = Vec::<u16>::new();
    resize_rgba8_srgb_gamma_correct_into(ResizeGammaInto {
        src_rgba8,
        src_stride_bytes: (src_w as usize).saturating_mul(4),
        src_x: 0,
        src_y: 0,
        src_w,
        src_h,
        dst_w,
        dst_h,
        filter,
        dst_rgba8: &mut out,
        dst_stride_bytes: (out_w as usize).saturating_mul(4),
        tmp_linear: &mut tmp,
    });
    out
}

pub fn resize_rgba8_srgb_gamma_correct_into(input: ResizeGammaInto<'_>) {
    let ResizeGammaInto {
        src_rgba8,
        src_stride_bytes,
        src_x,
        src_y,
        src_w,
        src_h,
        dst_w,
        dst_h,
        filter,
        dst_rgba8,
        dst_stride_bytes,
        tmp_linear,
    } = input;
    let out_w = dst_w.max(1);
    let out_h = dst_h.max(1);
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return;
    }
    let required_dst = dst_stride_bytes.saturating_mul(out_h as usize);
    if dst_rgba8.len() < required_dst {
        return;
    }

    let expected = (src_w as usize)
        .saturating_mul(src_h as usize)
        .saturating_mul(4);
    if tmp_linear.len() < expected {
        tmp_linear.resize(expected, 0);
    }
    for y in 0..src_h as usize {
        let src_row = (src_y as usize + y).saturating_mul(src_stride_bytes)
            + (src_x as usize).saturating_mul(4);
        let dst_row = y.saturating_mul(src_w as usize).saturating_mul(4);
        for x in 0..src_w as usize {
            let src_base = src_row + x * 4;
            let dst_base = dst_row + x * 4;
            if src_base + 3 >= src_rgba8.len() {
                continue;
            }
            let a_u16 = (src_rgba8[src_base + 3] as u32).saturating_mul(257) as u16;
            let r_lin = srgb_u8_to_linear_u16(src_rgba8[src_base]);
            let g_lin = srgb_u8_to_linear_u16(src_rgba8[src_base + 1]);
            let b_lin = srgb_u8_to_linear_u16(src_rgba8[src_base + 2]);

            let a32 = a_u16 as u32;
            let r_p = ((r_lin as u32).saturating_mul(a32) + 32767) / 65535;
            let g_p = ((g_lin as u32).saturating_mul(a32) + 32767) / 65535;
            let b_p = ((b_lin as u32).saturating_mul(a32) + 32767) / 65535;

            tmp_linear[dst_base] = r_p as u16;
            tmp_linear[dst_base + 1] = g_p as u16;
            tmp_linear[dst_base + 2] = b_p as u16;
            tmp_linear[dst_base + 3] = a_u16;
        }
    }

    let mut linear = std::mem::take(tmp_linear);
    linear.truncate(expected);
    metrics::record_cpu_resize_simd_attempt();
    let simd_start = Instant::now();
    let resized_u16 = if let Some(simd) =
        resize_rgba16_premultiplied_simd(&linear, src_w, src_h, out_w, out_h, filter)
    {
        metrics::record_cpu_resize_simd_ok(simd_start.elapsed().as_millis() as u64);
        *tmp_linear = linear;
        simd
    } else {
        metrics::record_cpu_resize_simd_fallback();
        let Some(src_img) = ImageBuffer::<Rgba<u16>, Vec<u16>>::from_raw(src_w, src_h, linear)
        else {
            return;
        };
        let resized: ImageBuffer<Rgba<u16>, Vec<u16>> =
            image::imageops::resize(&src_img, out_w, out_h, filter);
        *tmp_linear = src_img.into_raw();
        resized.into_raw()
    };

    for y in 0..out_h as usize {
        let dst_row = y.saturating_mul(dst_stride_bytes);
        let src_row = y.saturating_mul(out_w as usize).saturating_mul(4);
        for x in 0..out_w as usize {
            let base = src_row + x * 4;
            let out_base = dst_row + x * 4;
            let a_u16: u16 = resized_u16[base + 3];
            let a_u8 = ((a_u16 as u32 * 255 + 32767) / 65535) as u8;

            if a_u16 == 0 {
                dst_rgba8[out_base] = 0;
                dst_rgba8[out_base + 1] = 0;
                dst_rgba8[out_base + 2] = 0;
                dst_rgba8[out_base + 3] = 0;
                continue;
            }

            let inv_a = 1.0 / (a_u16 as f32);
            let r_lin = (resized_u16[base] as f32 * inv_a).clamp(0.0, 1.0);
            let g_lin = (resized_u16[base + 1] as f32 * inv_a).clamp(0.0, 1.0);
            let b_lin = (resized_u16[base + 2] as f32 * inv_a).clamp(0.0, 1.0);

            dst_rgba8[out_base] = linear_to_srgb_u8(r_lin);
            dst_rgba8[out_base + 1] = linear_to_srgb_u8(g_lin);
            dst_rgba8[out_base + 2] = linear_to_srgb_u8(b_lin);
            dst_rgba8[out_base + 3] = a_u8;
        }
    }
}

pub struct ResizeGammaInto<'a> {
    pub src_rgba8: &'a [u8],
    pub src_stride_bytes: usize,
    pub src_x: u32,
    pub src_y: u32,
    pub src_w: u32,
    pub src_h: u32,
    pub dst_w: u32,
    pub dst_h: u32,
    pub filter: FilterType,
    pub dst_rgba8: &'a mut [u8],
    pub dst_stride_bytes: usize,
    pub tmp_linear: &'a mut Vec<u16>,
}

fn resize_rgba8_srgb_gamma_correct_image(
    src: &RgbaImage,
    width: u32,
    height: u32,
    filter: FilterType,
) -> RgbaImage {
    if width == 0 || height == 0 {
        return ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 0]));
    }
    let (src_w, src_h) = src.dimensions();
    let bytes = resize_rgba8_srgb_gamma_correct(src.as_raw(), src_w, src_h, width, height, filter);
    ImageBuffer::from_raw(width, height, bytes)
        .unwrap_or_else(|| ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 0])))
}

fn gamma_correct_resize_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        let val = std::env::var("CANVAS_GAMMA_CORRECT_RESIZE")
            .unwrap_or_default()
            .trim()
            .to_lowercase();
        if val.is_empty() {
            return true;
        }
        if matches!(val.as_str(), "0" | "false" | "no" | "off") {
            return false;
        }
        if matches!(val.as_str(), "1" | "true" | "yes" | "on") {
            return true;
        }
        true
    })
}

pub fn gamma_correct_resize_enabled_value() -> bool {
    gamma_correct_resize_enabled()
}

fn pick_gamma_filter(src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> FilterType {
    if dst_w <= src_w && dst_h <= src_h {
        FilterType::Lanczos3
    } else {
        FilterType::CatmullRom
    }
}

fn resize_rgba16_premultiplied_simd(
    src_linear: &[u16],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
    filter: FilterType,
) -> Option<Vec<u16>> {
    let expected_src = (src_w as usize)
        .saturating_mul(src_h as usize)
        .saturating_mul(4);
    if src_linear.len() < expected_src {
        return None;
    }

    let src_bytes = unsafe {
        std::slice::from_raw_parts(
            src_linear.as_ptr() as *const u8,
            expected_src.saturating_mul(2),
        )
    };
    let src_img =
        fir::images::ImageRef::new(src_w, src_h, src_bytes, fir::PixelType::U16x4).ok()?;
    let mut dst_img = fir::images::Image::new(dst_w, dst_h, fir::PixelType::U16x4);

    let options = fir::ResizeOptions::new()
        .resize_alg(fir_resize_alg(filter))
        .use_alpha(false);

    let mut resizer = fir::Resizer::new();
    resizer
        .resize(&src_img, &mut dst_img, Some(&options))
        .ok()?;

    let dst_bytes = dst_img.into_vec();
    if dst_bytes.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(dst_bytes.len() / 2);
    for chunk in dst_bytes.chunks_exact(2) {
        out.push(u16::from_ne_bytes([chunk[0], chunk[1]]));
    }
    Some(out)
}

fn fir_resize_alg(filter: FilterType) -> fir::ResizeAlg {
    match filter {
        FilterType::Nearest => fir::ResizeAlg::Nearest,
        FilterType::Triangle => fir::ResizeAlg::Convolution(fir::FilterType::Bilinear),
        FilterType::CatmullRom => fir::ResizeAlg::Convolution(fir::FilterType::CatmullRom),
        FilterType::Gaussian => fir::ResizeAlg::Convolution(fir::FilterType::Gaussian),
        FilterType::Lanczos3 => fir::ResizeAlg::Convolution(fir::FilterType::Lanczos3),
    }
}

fn rgba8_to_linear_premultiplied(src: &RgbaImage) -> ImageBuffer<Rgba<f32>, Vec<f32>> {
    let (w, h) = src.dimensions();
    let mut out = vec![0.0f32; (w as usize).saturating_mul(h as usize).saturating_mul(4)];
    let src_samples = src.as_flat_samples();
    let src_bytes = src_samples.samples;
    let px = (w as usize).saturating_mul(h as usize);

    for i in 0..px {
        let base = i * 4;
        let a = (src_bytes[base + 3] as f32) * (1.0 / 255.0);
        let r = srgb_u8_to_linear(src_bytes[base]) * a;
        let g = srgb_u8_to_linear(src_bytes[base + 1]) * a;
        let b = srgb_u8_to_linear(src_bytes[base + 2]) * a;
        out[base] = r;
        out[base + 1] = g;
        out[base + 2] = b;
        out[base + 3] = a;
    }

    ImageBuffer::from_raw(w, h, out)
        .unwrap_or_else(|| ImageBuffer::from_pixel(1, 1, Rgba([0.0, 0.0, 0.0, 0.0])))
}

fn linear_premultiplied_to_rgba8(src: &ImageBuffer<Rgba<f32>, Vec<f32>>) -> RgbaImage {
    let (w, h) = src.dimensions();
    let mut out = vec![0u8; (w as usize).saturating_mul(h as usize).saturating_mul(4)];
    let src_samples = src.as_flat_samples();
    let src_f = src_samples.samples;
    let px = (w as usize).saturating_mul(h as usize);

    for i in 0..px {
        let base = i * 4;
        let a = src_f[base + 3].clamp(0.0, 1.0);
        let inv_a = if a > 0.0 { 1.0 / a } else { 0.0 };
        let r = (src_f[base] * inv_a).clamp(0.0, 1.0);
        let g = (src_f[base + 1] * inv_a).clamp(0.0, 1.0);
        let b = (src_f[base + 2] * inv_a).clamp(0.0, 1.0);
        out[base] = linear_to_srgb_u8(r);
        out[base + 1] = linear_to_srgb_u8(g);
        out[base + 2] = linear_to_srgb_u8(b);
        out[base + 3] = (a * 255.0 + 0.5).floor().clamp(0.0, 255.0) as u8;
    }

    ImageBuffer::from_raw(w, h, out)
        .unwrap_or_else(|| ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 0])))
}

fn srgb_u8_to_linear(v: u8) -> f32 {
    static LUT: OnceLock<[f32; 256]> = OnceLock::new();
    let table = LUT.get_or_init(|| {
        let mut out = [0.0f32; 256];
        for (i, val) in out.iter_mut().enumerate() {
            let s = i as f32 / 255.0;
            *val = if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            };
        }
        out
    });
    table[v as usize]
}

fn srgb_u8_to_linear_u16(v: u8) -> u16 {
    static LUT: OnceLock<[u16; 256]> = OnceLock::new();
    let table = LUT.get_or_init(|| {
        let mut out = [0u16; 256];
        for (i, val) in out.iter_mut().enumerate() {
            let s = i as f32 / 255.0;
            let lin = if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            };
            let v = (lin * 65535.0 + 0.5).floor().clamp(0.0, 65535.0) as u16;
            *val = v;
        }
        out
    });
    table[v as usize]
}

fn linear_to_srgb_u8(v: f32) -> u8 {
    let v = v.clamp(0.0, 1.0);
    let s = if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0 + 0.5).floor().clamp(0.0, 255.0) as u8
}
