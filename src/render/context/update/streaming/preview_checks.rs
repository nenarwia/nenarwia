use crate::render::atlas::ThumbTier;
use crate::render::context::state::RenderContext;
use crate::render::streaming::canvas_media_slots::calculator::media_world_size_to_pixels;
use crate::render::streaming::preview::{
    pick_enabled_tier, required_coverage_thumb_tier, thumb_ready, thumb_request_key,
};

pub(super) fn coverage_thumb_key_for_item(
    ctx: &RenderContext,
    id: u64,
    item_idx: usize,
) -> Option<crate::render::streaming::preview::ThumbRequestKey> {
    let (obj_px_w, obj_px_h) = media_world_size_to_pixels(ctx, item_idx)?;
    let max_px = obj_px_w.max(obj_px_h);
    let required = required_coverage_thumb_tier(max_px);
    let tier = pick_enabled_tier(&ctx.atlas, required);
    Some(thumb_request_key(id, tier))
}

pub(super) fn preview_has_any(ctx: &RenderContext, id: u64, item_idx: usize) -> bool {
    let Some(raw) = ctx.scene.all_items_raw.get(item_idx) else {
        return false;
    };
    if raw.uv_region[2] <= 0.0 {
        return false;
    }
    let Some(tier) = ThumbTier::decode_uv_x(raw.uv_region[0]) else {
        return false;
    };
    ctx.atlas.has(tier, id)
}

pub(super) fn preview_has_coverage(ctx: &RenderContext, id: u64, item_idx: usize) -> bool {
    if !preview_has_any(ctx, id, item_idx) {
        return false;
    }
    let Some((obj_px_w, obj_px_h)) = media_world_size_to_pixels(ctx, item_idx) else {
        return false;
    };
    let max_px = obj_px_w.max(obj_px_h);
    let required = required_coverage_thumb_tier(max_px);
    let required = pick_enabled_tier(&ctx.atlas, required);
    thumb_ready(ctx, item_idx, required) && ctx.atlas.best_available_uv(id).is_some()
}

#[inline]
pub(super) fn item_has_known_dimensions(ctx: &RenderContext, item_idx: usize) -> bool {
    ctx.scene
        .item_dimensions
        .get(item_idx)
        .map(|&(w, h)| w > 0 && h > 0)
        .unwrap_or(false)
}
