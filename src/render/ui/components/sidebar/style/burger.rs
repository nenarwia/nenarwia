use super::super::super::BURGER_BTN_SIZE_PX;
use super::super::state::BurgerTexture;
use super::constants::{BURGER_HOVER_INSET_PX, BURGER_HOVER_RADIUS_PX, SIDEBAR_HOVER_COLOR};
use super::primitives::{draw_horizontal_capsule_aa, fill_rounded_rect_region_aa};

pub(super) fn build_burger_texture(hovered: bool) -> BurgerTexture {
    // Render icon at higher internal resolution and downsample to button size.
    // This preserves rounded caps at small on-screen sizes (35px).
    let width = BURGER_BTN_SIZE_PX.saturating_mul(2);
    let height = BURGER_BTN_SIZE_PX.saturating_mul(2);
    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    let scale = width as f32 / 35.0;

    if hovered {
        let inset = ((BURGER_HOVER_INSET_PX * scale).round() as u32).min(width / 4);
        let hover_rect = [
            inset,
            inset,
            width.saturating_sub(inset.saturating_mul(2)),
            height.saturating_sub(inset.saturating_mul(2)),
        ];
        fill_rounded_rect_region_aa(
            &mut pixels,
            width,
            height,
            hover_rect,
            BURGER_HOVER_RADIUS_PX * scale,
            SIDEBAR_HOVER_COLOR,
        );
    }

    // Keep idle button transparent so it doesn't look permanently pressed.
    // Match old `Menu` style: thin bars with true round caps.
    let line_w = ((14.0 * scale).round() as u32).clamp(6, width.saturating_sub(2));
    let line_x0 = ((width.saturating_sub(line_w)) / 2) as f32;
    let line_x1 = line_x0 + line_w as f32;
    let line_radius = (1.0 * scale).max(1.0);
    let line_color = [255, 255, 255, 235];
    draw_horizontal_capsule_aa(
        &mut pixels,
        width,
        height,
        line_x0,
        line_x1,
        height as f32 * (11.0 / 35.0),
        line_radius,
        line_color,
    );
    draw_horizontal_capsule_aa(
        &mut pixels,
        width,
        height,
        line_x0,
        line_x1,
        height as f32 * (17.0 / 35.0),
        line_radius,
        line_color,
    );
    draw_horizontal_capsule_aa(
        &mut pixels,
        width,
        height,
        line_x0,
        line_x1,
        height as f32 * (23.0 / 35.0),
        line_radius,
        line_color,
    );

    BurgerTexture {
        pixels,
        width,
        height,
    }
}
