pub(super) fn blit_cover_bilinear(
    dst_pixels: &mut [u8],
    dst_width: u32,
    dst_height: u32,
    rect: [u32; 4],
    src_pixels: &[u8],
    src_width: u32,
    src_height: u32,
) {
    if dst_width == 0
        || dst_height == 0
        || src_width == 0
        || src_height == 0
        || rect[2] == 0
        || rect[3] == 0
    {
        return;
    }
    if src_pixels.len() < (src_width as usize) * (src_height as usize) * 4 {
        return;
    }

    let x0 = rect[0].min(dst_width);
    let y0 = rect[1].min(dst_height);
    let x1 = rect[0].saturating_add(rect[2]).min(dst_width);
    let y1 = rect[1].saturating_add(rect[3]).min(dst_height);
    if x1 <= x0 || y1 <= y0 {
        return;
    }

    let target_w = (x1 - x0).max(1);
    let target_h = (y1 - y0).max(1);
    let target_aspect = target_w as f32 / target_h as f32;
    let src_aspect = src_width as f32 / src_height as f32;

    let (crop_x, crop_y, crop_w, crop_h) = if target_aspect > src_aspect {
        let crop_h = ((src_width as f32 / target_aspect).round() as u32).clamp(1, src_height);
        let crop_y = (src_height.saturating_sub(crop_h)) / 2;
        (0u32, crop_y, src_width, crop_h)
    } else {
        let crop_w = ((src_height as f32 * target_aspect).round() as u32).clamp(1, src_width);
        let crop_x = (src_width.saturating_sub(crop_w)) / 2;
        (crop_x, 0u32, crop_w, src_height)
    };

    let sx = crop_w as f32 / target_w as f32;
    let sy = crop_h as f32 / target_h as f32;

    for dy in 0..target_h {
        let src_y = (dy as f32 + 0.5) * sy + crop_y as f32 - 0.5;
        let y_base = src_y.floor() as i32;
        let fy = src_y - y_base as f32;
        let y0c = y_base.clamp(0, src_height.saturating_sub(1) as i32) as u32;
        let y1c = (y_base + 1).clamp(0, src_height.saturating_sub(1) as i32) as u32;
        let dst_y = y0 + dy;

        for dx in 0..target_w {
            let src_x = (dx as f32 + 0.5) * sx + crop_x as f32 - 0.5;
            let x_base = src_x.floor() as i32;
            let fx = src_x - x_base as f32;
            let x0c = x_base.clamp(0, src_width.saturating_sub(1) as i32) as u32;
            let x1c = (x_base + 1).clamp(0, src_width.saturating_sub(1) as i32) as u32;
            let dst_x = x0 + dx;

            let idx00 = ((y0c * src_width + x0c) * 4) as usize;
            let idx10 = ((y0c * src_width + x1c) * 4) as usize;
            let idx01 = ((y1c * src_width + x0c) * 4) as usize;
            let idx11 = ((y1c * src_width + x1c) * 4) as usize;

            let w00 = (1.0 - fx) * (1.0 - fy);
            let w10 = fx * (1.0 - fy);
            let w01 = (1.0 - fx) * fy;
            let w11 = fx * fy;

            let out_idx = ((dst_y * dst_width + dst_x) * 4) as usize;
            for c in 0..4 {
                let v = src_pixels[idx00 + c] as f32 * w00
                    + src_pixels[idx10 + c] as f32 * w10
                    + src_pixels[idx01 + c] as f32 * w01
                    + src_pixels[idx11 + c] as f32 * w11;
                dst_pixels[out_idx + c] = v.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

pub(super) fn blit_rgba_region(
    dst_pixels: &mut [u8],
    dst_width: u32,
    dst_height: u32,
    dst_rect: [u32; 4],
    src_pixels: &[u8],
    src_width: u32,
    src_height: u32,
) {
    if src_width == 0 || src_height == 0 {
        return;
    }
    if src_pixels.len() < (src_width as usize) * (src_height as usize) * 4 {
        return;
    }

    let copy_w = dst_rect[2]
        .min(src_width)
        .min(dst_width.saturating_sub(dst_rect[0]));
    let copy_h = dst_rect[3]
        .min(src_height)
        .min(dst_height.saturating_sub(dst_rect[1]));
    if copy_w == 0 || copy_h == 0 {
        return;
    }

    for y in 0..copy_h {
        let dst_row = ((dst_rect[1] + y) * dst_width * 4 + dst_rect[0] * 4) as usize;
        let src_row = (y * src_width * 4) as usize;
        let len = (copy_w * 4) as usize;
        dst_pixels[dst_row..dst_row + len].copy_from_slice(&src_pixels[src_row..src_row + len]);
    }
}

pub(super) fn draw_circle_aa(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    cx: f32,
    cy: f32,
    radius: f32,
    color: [u8; 4],
) {
    if width == 0 || height == 0 || radius <= 0.0 {
        return;
    }
    let min_x = (cx - radius - 1.0).floor().max(0.0) as i32;
    let max_x = (cx + radius + 1.0)
        .ceil()
        .min(width.saturating_sub(1) as f32) as i32;
    let min_y = (cy - radius - 1.0).floor().max(0.0) as i32;
    let max_y = (cy + radius + 1.0)
        .ceil()
        .min(height.saturating_sub(1) as f32) as i32;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let dx = px - cx;
            let dy = py - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let coverage = (radius + 0.5 - dist).clamp(0.0, 1.0);
            if coverage <= 0.0 {
                continue;
            }
            let idx = (((y as u32) * width + (x as u32)) * 4) as usize;
            blend_pixel_coverage(&mut pixels[idx..idx + 4], color, coverage);
        }
    }
}

pub(super) fn fill_rect_region(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    rect: [u32; 4],
    color: [u8; 4],
) {
    let x0 = rect[0].min(width);
    let y0 = rect[1].min(height);
    let x1 = rect[0].saturating_add(rect[2]).min(width);
    let y1 = rect[1].saturating_add(rect[3]).min(height);
    for y in y0..y1 {
        let row = (y * width * 4) as usize;
        for x in x0..x1 {
            let idx = row + (x * 4) as usize;
            pixels[idx] = color[0];
            pixels[idx + 1] = color[1];
            pixels[idx + 2] = color[2];
            pixels[idx + 3] = color[3];
        }
    }
}

fn blend_pixel_coverage(dst: &mut [u8], src: [u8; 4], coverage: f32) {
    let src_a = (src[3] as f32 / 255.0) * coverage.clamp(0.0, 1.0);
    if src_a <= 0.0 {
        return;
    }

    let dst_a = dst[3] as f32 / 255.0;
    let out_a = src_a + dst_a * (1.0 - src_a);
    if out_a <= 0.0 {
        dst[0] = 0;
        dst[1] = 0;
        dst[2] = 0;
        dst[3] = 0;
        return;
    }

    for i in 0..3 {
        let src_c = src[i] as f32 / 255.0;
        let dst_c = dst[i] as f32 / 255.0;
        let out_c = (src_c * src_a + dst_c * dst_a * (1.0 - src_a)) / out_a;
        dst[i] = (out_c * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    dst[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
}
