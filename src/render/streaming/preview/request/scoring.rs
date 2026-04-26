use crate::render::context::state::RenderContext;

const PREVIEW_CENTER_MOVING_SCALE: f32 = 1.0;
const PREVIEW_CENTER_IDLE_SCALE: f32 = 0.45;

#[inline]
fn preview_center_closeness(ctx: &RenderContext, obj_x: f64, obj_y: f64) -> f32 {
    let (min_x, max_x, min_y, max_y) = ctx.view().viewport_rect();
    let center_x = (min_x + max_x) * 0.5;
    let center_y = (min_y + max_y) * 0.5;
    let half_w = ((max_x - min_x) * 0.5).max(1e-6);
    let half_h = ((max_y - min_y) * 0.5).max(1e-6);

    let nx = ((obj_x - center_x).abs() / half_w).min(1.0) as f32;
    let ny = ((obj_y - center_y).abs() / half_h).min(1.0) as f32;
    let ring = nx.max(ny);
    (1.0 - ring).clamp(0.0, 1.0)
}

#[inline]
pub(super) fn preview_center_bonus(
    ctx: &RenderContext,
    obj_x: f64,
    obj_y: f64,
    max_bonus: i32,
) -> i32 {
    let scale = if ctx.viewport_runtime().moving_recently {
        PREVIEW_CENTER_MOVING_SCALE
    } else {
        PREVIEW_CENTER_IDLE_SCALE
    };
    let closeness = preview_center_closeness(ctx, obj_x, obj_y);
    ((max_bonus as f32) * closeness * scale).round() as i32
}

#[inline]
fn thumb_effective_capacity_px(orig_w: u32, orig_h: u32, tier_px: f32) -> (f32, f32) {
    if tier_px <= 0.0 {
        return (0.0, 0.0);
    }
    if orig_w == 0 || orig_h == 0 {
        return (tier_px, tier_px);
    }

    let aspect = orig_w as f32 / orig_h as f32;
    if !aspect.is_finite() || aspect <= 0.0 {
        return (tier_px, tier_px);
    }

    let (fill_w, fill_h) = if aspect >= 1.0 {
        (1.0, (1.0 / aspect).clamp(0.0, 1.0))
    } else {
        (aspect.clamp(0.0, 1.0), 1.0)
    };
    ((tier_px * fill_w).max(1.0), (tier_px * fill_h).max(1.0))
}

#[inline]
pub(super) fn thumb_undersample_ratio(
    obj_px_w: f32,
    obj_px_h: f32,
    orig_w: u32,
    orig_h: u32,
    tier_px: f32,
) -> f32 {
    let (cap_w, cap_h) = thumb_effective_capacity_px(orig_w, orig_h, tier_px);
    if cap_w <= 0.0 || cap_h <= 0.0 {
        return f32::INFINITY;
    }
    (obj_px_w / cap_w).max(obj_px_h / cap_h).max(1.0)
}
