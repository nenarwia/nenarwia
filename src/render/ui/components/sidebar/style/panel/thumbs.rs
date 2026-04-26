pub(in crate::render::ui::sidebar::style::panel) fn inset_rect(
    rect: [u32; 4],
    inset: u32,
) -> [u32; 4] {
    let width = rect[2].saturating_sub(inset * 2).max(1);
    let height = rect[3].saturating_sub(inset * 2).max(1);
    [
        rect[0].saturating_add(inset),
        rect[1].saturating_add(inset),
        width,
        height,
    ]
}

pub(in crate::render::ui::sidebar::style::panel) fn blit_cover_bilinear(
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

#[cfg(test)]
mod tests {
    use super::blit_cover_bilinear;

    #[test]
    fn blit_cover_bilinear_ignores_invalid_source_buffer_and_zero_sized_inputs() {
        let original = vec![7u8; 4 * 4 * 4];

        let mut invalid_src_dst = original.clone();
        blit_cover_bilinear(&mut invalid_src_dst, 4, 4, [0, 0, 4, 4], &[1, 2, 3], 4, 4);
        assert_eq!(invalid_src_dst, original);

        let mut zero_sized_dst = original.clone();
        blit_cover_bilinear(&mut zero_sized_dst, 4, 4, [0, 0, 4, 4], &[1; 16], 0, 4);
        assert_eq!(zero_sized_dst, original);
    }
}
