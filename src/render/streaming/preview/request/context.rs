use crate::render::atlas::ThumbTier;
use crate::render::context::state::RenderContext;
use crate::render::streaming::canvas_media_slots::calculator::{
    media_world_geometry, media_world_size_to_pixels,
};

use crate::render::streaming::preview::{
    pick_enabled_tier, required_coverage_thumb_tier, required_thumb_tier,
};

use super::scoring::{preview_center_bonus, thumb_undersample_ratio};

const PREVIEW_CENTER_COVERAGE_BONUS_MAX: i32 = 240_000;
const PREVIEW_CENTER_QUALITY_BONUS_MAX: i32 = 110_000;

#[derive(Clone, Copy)]
pub(super) struct PreviewRequestContext {
    pub asset_key: u64,
    pub obj_x: f64,
    pub obj_y: f64,
    pub obj_w: f32,
    pub obj_h: f32,
    pub obj_px_w: f32,
    pub obj_px_h: f32,
    pub max_px: f32,
    pub orig_w: u32,
    pub orig_h: u32,
    pub thumb_prio_coverage: i32,
    pub thumb_prio_quality: i32,
    pub coverage_tier: ThumbTier,
    pub desired_tier: ThumbTier,
    pub desired_tier_px: f32,
    pub thumb_undersample: f32,
    pub thumb_undersampled: bool,
}

pub(super) fn build_preview_request_context(
    ctx: &RenderContext,
    item_idx: usize,
) -> Option<PreviewRequestContext> {
    let asset_key = ctx.scene.asset_keys.get(item_idx).copied().unwrap_or(0);
    let (obj_x, obj_y, obj_w, obj_h) = media_world_geometry(ctx, item_idx)?;

    let (obj_px_w, obj_px_h) = media_world_size_to_pixels(ctx, item_idx)?;
    let max_px = obj_px_w.max(obj_px_h);
    let debt = ctx.scene.quality_debt.get(item_idx).copied().unwrap_or(0.0);
    let prio_px = (max_px * 100.0).min(i32::MAX as f32) as i32;
    let prio_debt = (debt * 1000.0) as i32;
    let thumb_prio = prio_px.saturating_add(prio_debt);
    let thumb_prio_coverage = thumb_prio.saturating_add(preview_center_bonus(
        ctx,
        obj_x,
        obj_y,
        PREVIEW_CENTER_COVERAGE_BONUS_MAX,
    ));
    let thumb_prio_quality = thumb_prio.saturating_add(preview_center_bonus(
        ctx,
        obj_x,
        obj_y,
        PREVIEW_CENTER_QUALITY_BONUS_MAX,
    ));

    let (orig_w, orig_h) = ctx
        .scene
        .item_dimensions
        .get(item_idx)
        .copied()
        .unwrap_or((0, 0));
    let required_tier = required_thumb_tier(max_px);
    let coverage_required_tier = required_coverage_thumb_tier(max_px);
    let coverage_tier = pick_enabled_tier(&ctx.atlas, coverage_required_tier);
    let desired_tier = pick_enabled_tier(&ctx.atlas, required_tier);
    let desired_tier_px = desired_tier.page_size() as f32;
    let thumb_undersample =
        thumb_undersample_ratio(obj_px_w, obj_px_h, orig_w, orig_h, desired_tier_px);
    let thumb_undersampled = thumb_undersample > 1.0;

    Some(PreviewRequestContext {
        asset_key,
        obj_x,
        obj_y,
        obj_w,
        obj_h,
        obj_px_w,
        obj_px_h,
        max_px,
        orig_w,
        orig_h,
        thumb_prio_coverage,
        thumb_prio_quality,
        coverage_tier,
        desired_tier,
        desired_tier_px,
        thumb_undersample,
        thumb_undersampled,
    })
}
