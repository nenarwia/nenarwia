use winit::dpi::PhysicalSize;

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

pub(super) fn draw_horizontal_capsule_aa(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    x0: f32,
    x1: f32,
    y_center: f32,
    radius: f32,
    color: [u8; 4],
) {
    if width == 0 || height == 0 || radius <= 0.0 || x1 <= x0 {
        return;
    }

    let min_x = (x0 - radius - 1.0).floor().max(0.0) as i32;
    let max_x = (x1 + radius + 1.0)
        .ceil()
        .min(width.saturating_sub(1) as f32) as i32;
    let min_y = (y_center - radius - 1.0).floor().max(0.0) as i32;
    let max_y = (y_center + radius + 1.0)
        .ceil()
        .min(height.saturating_sub(1) as f32) as i32;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let closest_x = px.clamp(x0, x1);
            let dx = px - closest_x;
            let dy = py - y_center;
            let dist = (dx * dx + dy * dy).sqrt();
            let coverage = (radius + 0.5 - dist).clamp(0.0, 1.0);
            if coverage > 0.0 {
                let idx = (((y as u32) * width + (x as u32)) * 4) as usize;
                blend_pixel_coverage(&mut pixels[idx..idx + 4], color, coverage);
            }
        }
    }
}

pub(super) fn write_rect_vertices(
    queue: &wgpu::Queue,
    vertex_buffer: &wgpu::Buffer,
    surface_size: PhysicalSize<u32>,
    rect: [f32; 4],
) {
    super::super::super::geometry::write_rect_vertices(queue, vertex_buffer, surface_size, rect);
}

pub(super) fn ease_in_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let inv = -2.0 * t + 2.0;
        1.0 - (inv * inv * inv) * 0.5
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
