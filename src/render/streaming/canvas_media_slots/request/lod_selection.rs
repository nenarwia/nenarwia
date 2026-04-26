use crate::render::cache::math;
use crate::render::context::state::RenderContext;

use super::super::calculator::{
    cap_lod_for_screen, choose_lod_hysteresis, lod_info, required_lod_for_screen,
    QUALITY_DEBT_BUCKET_SCALE, QUALITY_DEBT_BUCKET_WEIGHT,
};
use super::super::visibility::visible_tiles_stats;
use crate::render::streaming::common::quality::{record_ttfq_if_ready, update_quality_debt};

pub(super) struct DesiredLodState {
    pub desired_lod: u8,
    pub max_lod: u8,
    pub cap_lod: Option<u8>,
    pub desired_w: u32,
    pub desired_h: u32,
    pub desired_vis: Option<math::VisibleTiles>,
    pub desired_tiles_x: u32,
    pub desired_tiles_y: u32,
    pub desired_tiles_x_f: f32,
    pub desired_tiles_y_f: f32,
    pub desired_complete: bool,
    pub debt_boost: i32,
}

#[derive(Clone, Copy)]
pub(super) struct LodSelectionInput {
    pub id: u64,
    pub item_idx: usize,
    pub asset_key: u64,
    pub orig_w: u32,
    pub orig_h: u32,
    pub obj_x: f64,
    pub obj_y: f64,
    pub obj_w: f32,
    pub obj_h: f32,
    pub obj_px_w: f32,
    pub obj_px_h: f32,
    pub full: bool,
}

pub(super) fn compute_desired_lod_state(
    ctx: &mut RenderContext,
    input: LodSelectionInput,
) -> DesiredLodState {
    let item_idx = input.item_idx;
    let prev_lod = ctx.scene.last_lod.get(item_idx).copied().unwrap_or(0);
    let required_lod =
        required_lod_for_screen(input.orig_w, input.orig_h, input.obj_px_w, input.obj_px_h);
    let cap_lod = cap_lod_for_screen(input.orig_w, input.orig_h, input.obj_px_w, input.obj_px_h)
        .filter(|cap| *cap <= required_lod);
    let (mut desired_lod, max_lod) = choose_lod_hysteresis(
        input.orig_w,
        input.orig_h,
        input.obj_px_w,
        input.obj_px_h,
        prev_lod,
    );

    if desired_lod > required_lod {
        let (check_w, check_h, _, _, _, _) = lod_info(input.orig_w, input.orig_h, desired_lod);
        let undersample = (input.obj_px_w / check_w as f32).max(input.obj_px_h / check_h as f32);
        if input.full {
            ctx.quality_stats.record_lod_clamp(undersample);
        }
        desired_lod = required_lod;
    }

    if let Some(cap) = cap_lod {
        let cap = cap.min(max_lod);
        if desired_lod < cap {
            desired_lod = cap;
        }
    }

    if item_idx < ctx.scene.last_lod.len() {
        ctx.scene.last_lod[item_idx] = desired_lod;
    }

    let view = ctx.view().viewport_rect();
    let (
        desired_w,
        desired_h,
        desired_tiles_x,
        desired_tiles_y,
        desired_tiles_x_f,
        desired_tiles_y_f,
    ) = lod_info(input.orig_w, input.orig_h, desired_lod);

    let desired_vis = math::calculate_visible_tiles_f64(
        view,
        input.obj_x,
        input.obj_y,
        input.obj_w,
        input.obj_h,
        desired_w,
        desired_h,
    );

    let mut desired_complete = false;
    if input.full {
        let desired_stats = desired_vis
            .as_ref()
            .map(|v| visible_tiles_stats(ctx, input.asset_key, desired_lod, v))
            .unwrap_or_default();
        desired_complete = desired_stats.missing == 0;
        ctx.quality_stats
            .record_visible_tiles(desired_stats.missing, desired_stats.total);
        ctx.quality_stats
            .record_visible_tiles_last(desired_stats.missing, desired_stats.total);
        update_quality_debt(ctx, item_idx, desired_stats.ratio());
        if desired_stats.total > 0 {
            record_ttfq_if_ready(ctx, input.id, desired_complete);
        }
    }

    let quality_debt = ctx.scene.quality_debt.get(item_idx).copied().unwrap_or(0.0);
    let debt_bucket = (quality_debt * QUALITY_DEBT_BUCKET_SCALE).round().max(0.0) as i32;
    let debt_boost = debt_bucket.saturating_mul(QUALITY_DEBT_BUCKET_WEIGHT);

    DesiredLodState {
        desired_lod,
        max_lod,
        cap_lod,
        desired_w,
        desired_h,
        desired_vis,
        desired_tiles_x,
        desired_tiles_y,
        desired_tiles_x_f,
        desired_tiles_y_f,
        desired_complete,
        debt_boost,
    }
}
