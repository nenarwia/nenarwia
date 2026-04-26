pub(super) const NOTICE_RADIUS_PX: f32 = 14.0;
pub(super) const NOTICE_BG_COLOR: [u8; 4] = [8, 9, 12, 242];

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
    if x1 <= x0 || y1 <= y0 {
        return;
    }

    let row_stride = (width * 4) as usize;
    for y in y0..y1 {
        let row_start = y as usize * row_stride;
        for x in x0..x1 {
            let idx = row_start + (x as usize * 4);
            pixels[idx] = color[0];
            pixels[idx + 1] = color[1];
            pixels[idx + 2] = color[2];
            pixels[idx + 3] = color[3];
        }
    }
}

pub(super) fn fill_rounded_rect_region_aa(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    rect: [u32; 4],
    radius: f32,
    color: [u8; 4],
) {
    let x0 = rect[0].min(width);
    let y0 = rect[1].min(height);
    let x1 = rect[0].saturating_add(rect[2]).min(width);
    let y1 = rect[1].saturating_add(rect[3]).min(height);
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    if radius <= 0.0 {
        fill_rect_region(pixels, width, height, [x0, y0, x1 - x0, y1 - y0], color);
        return;
    }

    let rect_w = (x1 - x0) as f32;
    let rect_h = (y1 - y0) as f32;
    let r = radius.min(rect_w * 0.5).min(rect_h * 0.5).max(0.0);
    if r <= 0.0 {
        fill_rect_region(pixels, width, height, [x0, y0, x1 - x0, y1 - y0], color);
        return;
    }

    let cx0 = x0 as f32 + r;
    let cx1 = x1 as f32 - r;
    let cy0 = y0 as f32 + r;
    let cy1 = y1 as f32 - r;

    for y in y0..y1 {
        for x in x0..x1 {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let closest_x = px.clamp(cx0, cx1);
            let closest_y = py.clamp(cy0, cy1);
            let dx = px - closest_x;
            let dy = py - closest_y;
            let dist = (dx * dx + dy * dy).sqrt();
            let coverage = (r + 0.5 - dist).clamp(0.0, 1.0);
            if coverage <= 0.0 {
                continue;
            }
            let idx = (((y * width) + x) * 4) as usize;
            blend_pixel_coverage(&mut pixels[idx..idx + 4], color, coverage);
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

    for idx in 0..3 {
        let src_c = src[idx] as f32 / 255.0;
        let dst_c = dst[idx] as f32 / 255.0;
        let out_c = (src_c * src_a + dst_c * dst_a * (1.0 - src_a)) / out_a;
        dst[idx] = (out_c * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    dst[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
}

#[cfg(test)]
mod tests {
    use super::{blend_pixel_coverage, fill_rect_region, fill_rounded_rect_region_aa};

    #[test]
    fn fill_rounded_rect_region_with_zero_radius_matches_plain_fill() {
        let mut rounded = vec![0u8; 8 * 8 * 4];
        let mut plain = vec![0u8; 8 * 8 * 4];
        let rect = [1, 2, 5, 3];
        let color = [10, 20, 30, 200];

        fill_rounded_rect_region_aa(&mut rounded, 8, 8, rect, 0.0, color);
        fill_rect_region(&mut plain, 8, 8, rect, color);

        assert_eq!(rounded, plain);
    }

    #[test]
    fn blend_pixel_coverage_keeps_pixel_when_coverage_is_zero() {
        let mut dst = [12u8, 34, 56, 78];

        blend_pixel_coverage(&mut dst, [255, 255, 255, 255], 0.0);

        assert_eq!(dst, [12, 34, 56, 78]);
    }
}
