use crate::render::cache::math;
use crate::render::context::state::RenderContext;

use super::super::scheduler::{
    schedule_canvas_media_slots_for_lod, schedule_visible_canvas_media_slots_for_lod,
    touch_visible_canvas_media_slots_for_lod, ScheduleCanvasMediaSlotLodInput,
};

pub(super) struct SchedulingInput<'a> {
    pub id: u64,
    pub item_idx: usize,
    pub asset_key: u64,
    pub render_lod: u8,
    pub desired_lod: u8,
    pub render_vis: &'a Option<math::VisibleTiles>,
    pub desired_vis: &'a Option<math::VisibleTiles>,
    pub desired_complete: bool,
    pub coarse_vis: &'a Option<math::VisibleTiles>,
    pub coarse_lod: u8,
    pub coarse_tiles_x: u32,
    pub coarse_tiles_y: u32,
    pub desired_tiles_x: u32,
    pub desired_tiles_y: u32,
    pub debt_boost: i32,
    pub feedback_max_latency_frames: u64,
}

pub(super) fn schedule_tile_requests(ctx: &mut RenderContext, input: SchedulingInput<'_>) {
    let render_lod = input.render_lod;
    let desired_lod = input.desired_lod;
    let debt_boost = input.debt_boost;
    let feedback_rt = ctx
        .gpu_feedback
        .as_ref()
        .map(|fb| fb.is_rt())
        .unwrap_or(false);
    let use_feedback = ctx.streaming.use_gpu_feedback
        && feedback_rt
        && ctx.feedback.has_results
        && !ctx.feedback.overflow_last
        && ctx
            .frame_count
            .saturating_sub(ctx.feedback.last_ready_frame)
            <= input.feedback_max_latency_frames;

    if render_lod != desired_lod {
        if let Some(rv) = input.render_vis.as_ref() {
            schedule_visible_canvas_media_slots_for_lod(
                ctx,
                input.id,
                input.item_idx,
                render_lod,
                rv,
                2_800_000 + debt_boost,
            );
        }
    }

    if let Some(v) = input.desired_vis.as_ref() {
        if !input.desired_complete {
            if let Some(cvis) = input.coarse_vis.as_ref() {
                schedule_canvas_media_slots_for_lod(
                    ctx,
                    ScheduleCanvasMediaSlotLodInput {
                        id: input.id,
                        item_idx: input.item_idx,
                        lod: input.coarse_lod,
                        tiles: cvis,
                        total_tiles_x: input.coarse_tiles_x,
                        total_tiles_y: input.coarse_tiles_y,
                        base_visible: 2_500_000 + debt_boost,
                        base_prefetch: 1_500_000 + debt_boost,
                    },
                );
            }
            if !use_feedback {
                schedule_canvas_media_slots_for_lod(
                    ctx,
                    ScheduleCanvasMediaSlotLodInput {
                        id: input.id,
                        item_idx: input.item_idx,
                        lod: desired_lod,
                        tiles: v,
                        total_tiles_x: input.desired_tiles_x,
                        total_tiles_y: input.desired_tiles_y,
                        base_visible: 1_500_000 + debt_boost,
                        base_prefetch: 800_000 + debt_boost,
                    },
                );
            }
        } else if !use_feedback {
            schedule_canvas_media_slots_for_lod(
                ctx,
                ScheduleCanvasMediaSlotLodInput {
                    id: input.id,
                    item_idx: input.item_idx,
                    lod: desired_lod,
                    tiles: v,
                    total_tiles_x: input.desired_tiles_x,
                    total_tiles_y: input.desired_tiles_y,
                    base_visible: 2_000_000 + debt_boost,
                    base_prefetch: 1_000_000 + debt_boost,
                },
            );
        }
    }

    if use_feedback && render_lod == desired_lod {
        if let Some(v) = input.desired_vis.as_ref() {
            touch_visible_canvas_media_slots_for_lod(ctx, input.asset_key, render_lod, v);
        }
    }
}
