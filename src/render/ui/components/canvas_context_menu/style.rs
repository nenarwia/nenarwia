use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use winit::dpi::PhysicalSize;

use super::state::MenuTexture;

const MENU_WIDTH_PX: u32 = 228;
const MENU_HEIGHT_PX: u32 = 104;
const PANEL_RADIUS_PX: f32 = 18.0;
const PANEL_BG: [u8; 4] = [23, 23, 23, 178];
const PANEL_BORDER: [u8; 4] = [255, 255, 255, 24];
const HOVER_BG: [u8; 4] = [255, 255, 255, 20];
const TEXT_COLOR: [u8; 4] = [236, 236, 236, 255];
const TEXT_BUSY_COLOR: [u8; 4] = [236, 236, 236, 120];
const ROW_RADIUS_PX: f32 = 12.0;
const ROW_PADDING_X_PX: u32 = 18;
const ROW_TOP_PX: u32 = 10;
const ROW_HEIGHT_PX: u32 = 40;
const ROW_GAP_PX: u32 = 4;

pub(super) fn menu_width_px() -> u32 {
    MENU_WIDTH_PX
}

pub(super) fn menu_height_px() -> u32 {
    MENU_HEIGHT_PX
}

pub(super) fn build_menu_texture(
    hovered_show_in_explorer: bool,
    hovered_delete: bool,
    busy: bool,
) -> MenuTexture {
    let width = MENU_WIDTH_PX;
    let height = MENU_HEIGHT_PX;
    let show_in_explorer_rect = [10, ROW_TOP_PX, width.saturating_sub(20), ROW_HEIGHT_PX];
    let delete_rect = [
        10,
        ROW_TOP_PX + ROW_HEIGHT_PX + ROW_GAP_PX,
        width.saturating_sub(20),
        ROW_HEIGHT_PX,
    ];

    let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
    fill_rounded_rect_region_aa(
        &mut pixels,
        width,
        height,
        [0, 0, width, height],
        PANEL_RADIUS_PX,
        PANEL_BG,
    );
    fill_rounded_rect_region_aa(
        &mut pixels,
        width,
        height,
        [1, 1, width.saturating_sub(2), height.saturating_sub(2)],
        (PANEL_RADIUS_PX - 1.0).max(0.0),
        PANEL_BORDER,
    );
    fill_rounded_rect_region_aa(
        &mut pixels,
        width,
        height,
        [2, 2, width.saturating_sub(4), height.saturating_sub(4)],
        (PANEL_RADIUS_PX - 2.0).max(0.0),
        PANEL_BG,
    );

    if hovered_show_in_explorer && !busy {
        fill_rounded_rect_region_aa(
            &mut pixels,
            width,
            height,
            show_in_explorer_rect,
            ROW_RADIUS_PX,
            HOVER_BG,
        );
    }

    if hovered_delete && !busy {
        fill_rounded_rect_region_aa(
            &mut pixels,
            width,
            height,
            delete_rect,
            ROW_RADIUS_PX,
            HOVER_BG,
        );
    }

    if let Some(font) = super::super::font::load_font() {
        let scale = PxScale::from(17.0);
        let text_color = if busy { TEXT_BUSY_COLOR } else { TEXT_COLOR };
        draw_menu_row_label(
            &mut pixels,
            width,
            height,
            &font,
            scale,
            show_in_explorer_rect,
            "Show in Explorer",
            text_color,
        );
        draw_menu_row_label(
            &mut pixels,
            width,
            height,
            &font,
            scale,
            delete_rect,
            "Move to Trash",
            text_color,
        );
    }

    MenuTexture {
        pixels,
        width,
        height,
        show_in_explorer_rect,
        delete_rect,
    }
}

pub(super) fn write_rect_vertices(
    queue: &wgpu::Queue,
    vertex_buffer: &wgpu::Buffer,
    surface_size: PhysicalSize<u32>,
    rect: [f32; 4],
) {
    super::super::geometry::write_rect_vertices(queue, vertex_buffer, surface_size, rect);
}

fn draw_menu_row_label(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    font: &FontArc,
    scale: PxScale,
    rect: [u32; 4],
    text: &str,
    color: [u8; 4],
) {
    let scaled = font.as_scaled(scale);
    let ascent = scaled.ascent();
    let text_y =
        rect[1] as f32 + ((rect[3] as f32 - (scaled.ascent() - scaled.descent())) * 0.5) + ascent
            - 1.0;
    super::super::raster::draw_text_line(super::super::raster::TextLineParams {
        pixels,
        width,
        height,
        font,
        scale,
        x: (rect[0] + ROW_PADDING_X_PX) as f32,
        y: text_y,
        text,
        color,
    });
}

fn fill_rounded_rect_region_aa(
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

    let rect_w = (x1 - x0) as f32;
    let rect_h = (y1 - y0) as f32;
    let r = radius.min(rect_w * 0.5).min(rect_h * 0.5).max(0.0);
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

    for i in 0..3 {
        let src_c = src[i] as f32 / 255.0;
        let dst_c = dst[i] as f32 / 255.0;
        let out_c = (src_c * src_a + dst_c * dst_a * (1.0 - src_a)) / out_a;
        dst[i] = (out_c * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    dst[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
}
