use crate::render::atlas::ThumbTier;
use crate::render::context::state::RenderContext;
use crate::render::streaming::common::quality::{record_ttfq_if_ready, update_quality_debt};
use crate::render::streaming::common::sampling::sample_mode;
use crate::render::streaming::gpu_sync::{
    update_instance_params_desired_only, update_instance_params_if_changed, InstanceLayerParams,
    InstanceParamUpdate,
};

use super::{max_enabled_tier, request_thumbnail_if_needed, thumb_ready};

#[derive(Clone, Copy)]
pub(crate) struct EarlyRequestInput {
    pub id: u64,
    pub item_idx: usize,
    pub full: bool,
    pub allow_requests: bool,
    pub allow_quality_requests: bool,
    pub thumb_prio_coverage: i32,
    pub thumb_prio_quality: i32,
    pub coverage_tier: ThumbTier,
    pub desired_tier: ThumbTier,
    pub desired_tier_px: f32,
    pub max_px: f32,
    pub thumb_undersampled: bool,
}

pub(crate) fn handle_missing_dimensions(
    ctx: &mut RenderContext,
    input: EarlyRequestInput,
    orig_w: u32,
    orig_h: u32,
) -> bool {
    let item_idx = input.item_idx;
    let full = input.full;
    if orig_w != 0 && orig_h != 0 {
        return false;
    }

    if input.allow_requests {
        request_thumbnail_if_needed(
            ctx,
            input.id,
            item_idx,
            input.thumb_prio_coverage,
            input.thumb_prio_quality,
            input.allow_quality_requests,
            input.coverage_tier,
            input.desired_tier,
        );
    }
    let ready = thumb_ready(ctx, item_idx, input.desired_tier);
    if full {
        ctx.quality_stats
            .record_visible_tiles(if ready { 0 } else { 1 }, 1);
        update_quality_debt(ctx, item_idx, if ready { 0.0 } else { 1.0 });
        record_ttfq_if_ready(ctx, input.id, ready);
    }

    if full {
        let undersample = if input.desired_tier_px > 0.0 {
            (input.max_px / input.desired_tier_px).max(1.0)
        } else {
            1.0
        };
        ctx.quality_stats.record_visible_undersample(undersample);
    }

    if item_idx < ctx.scene.all_items_raw.len() {
        let mode = sample_mode(input.thumb_undersampled);
        ctx.scene.all_items_raw[item_idx].sample_flags = [mode, mode, mode, 0.0];
    }
    reset_item_lods(ctx, item_idx);
    true
}

pub(crate) fn handle_atlas_path(ctx: &mut RenderContext, input: EarlyRequestInput) -> bool {
    let item_idx = input.item_idx;
    let full = input.full;
    if input.allow_requests {
        request_thumbnail_if_needed(
            ctx,
            input.id,
            item_idx,
            input.thumb_prio_coverage,
            input.thumb_prio_quality,
            input.allow_quality_requests,
            input.coverage_tier,
            input.desired_tier,
        );
    }
    let thumb_is_ready = thumb_ready(ctx, item_idx, input.desired_tier);

    let was_tiled = ctx
        .scene
        .all_items_raw
        .get(item_idx)
        .map(|r| r.params[0] >= 0.0)
        .unwrap_or(false);

    let max_enabled = max_enabled_tier(&ctx.atlas);
    let atlas_limit_px = max_enabled.page_size() as f32;
    let atlas_allowed = input.max_px <= atlas_limit_px;
    // Atlas is a preview path. Keep it for small on-screen sizes only; larger items
    // must enter tile streaming to avoid "stuck blurry" states under tier churn.
    let enter_px = atlas_limit_px.min(128.0);
    let exit_px = (enter_px * 0.75).min(enter_px);
    let use_atlas = if was_tiled {
        !input.thumb_undersampled && atlas_allowed && input.max_px <= exit_px
    } else {
        !input.thumb_undersampled && atlas_allowed && input.max_px <= enter_px
    };

    if use_atlas && !thumb_is_ready {
        let atlas_mode = sample_mode(input.thumb_undersampled);
        let sample_flags = [atlas_mode, atlas_mode, atlas_mode, 0.0];
        if full {
            update_instance_params_if_changed(
                ctx,
                InstanceParamUpdate {
                    item_idx,
                    desired: InstanceLayerParams {
                        region: None,
                        tiles_x: 0.0,
                        tiles_y: 0.0,
                    },
                    coarse: InstanceLayerParams {
                        region: None,
                        tiles_x: 0.0,
                        tiles_y: 0.0,
                    },
                    sample_flags,
                },
            );
            ctx.quality_stats.record_visible_tiles(1, 1);
            update_quality_debt(ctx, item_idx, 1.0);
        } else {
            update_instance_params_desired_only(
                ctx,
                item_idx,
                InstanceLayerParams {
                    region: None,
                    tiles_x: 0.0,
                    tiles_y: 0.0,
                },
                sample_flags,
            );
        }
        reset_item_lods(ctx, item_idx);
        return true;
    }

    if !use_atlas {
        return false;
    }

    if full {
        let undersample = if input.desired_tier_px > 0.0 {
            (input.max_px / input.desired_tier_px).max(1.0)
        } else {
            1.0
        };
        ctx.quality_stats.record_visible_undersample(undersample);
    }

    let atlas_mode = sample_mode(input.thumb_undersampled);
    let sample_flags = [atlas_mode, atlas_mode, atlas_mode, 0.0];
    if full {
        update_instance_params_if_changed(
            ctx,
            InstanceParamUpdate {
                item_idx,
                desired: InstanceLayerParams {
                    region: None,
                    tiles_x: 0.0,
                    tiles_y: 0.0,
                },
                coarse: InstanceLayerParams {
                    region: None,
                    tiles_x: 0.0,
                    tiles_y: 0.0,
                },
                sample_flags,
            },
        );
        ctx.quality_stats
            .record_visible_tiles(if thumb_is_ready { 0 } else { 1 }, 1);
        update_quality_debt(ctx, item_idx, if thumb_is_ready { 0.0 } else { 1.0 });
        record_ttfq_if_ready(ctx, input.id, thumb_is_ready);
    } else {
        update_instance_params_desired_only(
            ctx,
            item_idx,
            InstanceLayerParams {
                region: None,
                tiles_x: 0.0,
                tiles_y: 0.0,
            },
            sample_flags,
        );
    }

    reset_item_lods(ctx, item_idx);
    true
}

fn reset_item_lods(ctx: &mut RenderContext, item_idx: usize) {
    if item_idx < ctx.scene.render_lod.len() {
        ctx.scene.render_lod[item_idx] = u8::MAX;
    }
    if item_idx < ctx.scene.coarse_lod.len() {
        ctx.scene.coarse_lod[item_idx] = u8::MAX;
    }
}
