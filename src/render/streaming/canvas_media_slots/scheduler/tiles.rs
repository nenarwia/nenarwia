use crate::render::cache::{math, CanvasMediaSlotId};
use crate::render::context::state::RenderContext;

use super::super::calculator::{
    lod_info, media_world_geometry, tile_center_distance_px, tile_tiebreak_bias,
    viewport_dist_penalty, TileDistanceInput, VIEWPORT_DIST_PREFETCH_MAX_PENALTY,
    VIEWPORT_DIST_VISIBLE_MAX_PENALTY, VIEWPORT_DIST_WEIGHT_PREFETCH, VIEWPORT_DIST_WEIGHT_VISIBLE,
};
use super::super::CanvasMediaSlotQueueItem;

use super::enqueue_canvas_media_slot_request;

pub struct ScheduleCanvasMediaSlotLodInput<'a> {
    pub id: u64,
    pub item_idx: usize,
    pub lod: u8,
    pub tiles: &'a math::VisibleTiles,
    pub total_tiles_x: u32,
    pub total_tiles_y: u32,
    pub base_visible: i32,
    pub base_prefetch: i32,
}

pub fn schedule_canvas_media_slots_for_lod(
    ctx: &mut RenderContext,
    input: ScheduleCanvasMediaSlotLodInput<'_>,
) {
    let ScheduleCanvasMediaSlotLodInput {
        id,
        item_idx,
        lod,
        tiles,
        total_tiles_x,
        total_tiles_y,
        base_visible,
        base_prefetch,
    } = input;

    let max_len = ctx.streaming.max_canvas_media_slot_queue_len.max(1);
    let total_len = ctx.streaming_runtime.canvas_media_slots.queue_visible.len()
        + ctx
            .streaming_runtime
            .canvas_media_slots
            .queue_prefetch
            .len();
    if total_len >= max_len
        && ctx
            .streaming_runtime
            .canvas_media_slots
            .queue_prefetch
            .is_empty()
    {
        return;
    }

    let asset_key = ctx.scene.asset_keys.get(item_idx).copied().unwrap_or(0);
    if !crate::core::loader::mem_cache::is_ram_media_slot_asset(asset_key) {
        return;
    }
    let Some((obj_x, obj_y, obj_w, obj_h)) = media_world_geometry(ctx, item_idx) else {
        return;
    };
    let Some(region) = ctx.page_directory.get_region(asset_key, lod) else {
        return;
    };
    let total_tiles_x = total_tiles_x.min(region.w);
    let total_tiles_y = total_tiles_y.min(region.h);
    if total_tiles_x == 0 || total_tiles_y == 0 {
        return;
    }
    let vis_min_tx = tiles.min_tx.min(total_tiles_x);
    let vis_max_tx = tiles.max_tx.min(total_tiles_x);
    let vis_min_ty = tiles.min_ty.min(total_tiles_y);
    let vis_max_ty = tiles.max_ty.min(total_tiles_y);
    if vis_min_tx >= vis_max_tx || vis_min_ty >= vis_max_ty {
        return;
    }

    let moving_recently = ctx.viewport_runtime().moving_recently;
    let r = if moving_recently {
        0
    } else {
        ctx.streaming.prefetch_radius_tiles
    };
    let ex_min_tx = vis_min_tx.saturating_sub(r);
    let ex_max_tx = vis_max_tx.saturating_add(r).min(total_tiles_x);
    let ex_min_ty = vis_min_ty.saturating_sub(r);
    let ex_max_ty = vis_max_ty.saturating_add(r).min(total_tiles_y);

    let mut request_canvas_media_slot = |tx: u32, ty: u32, base_prio: i32, is_prefetch: bool| {
        let tile_id = CanvasMediaSlotId {
            asset_key,
            lod,
            x: tx,
            y: ty,
        };

        if ctx.page_table.get_slot(tile_id).is_some() {
            ctx.page_table.touch(tile_id);
            return;
        }

        if ctx
            .streaming_runtime
            .canvas_media_slots
            .pending
            .get(&tile_id)
            .copied()
            == Some(ctx.streaming_runtime.stream_epoch)
        {
            return;
        }

        if is_prefetch && moving_recently {
            return;
        }

        if is_prefetch
            && ctx
                .streaming_runtime
                .budgets
                .canvas_media_slot_budget_remaining
                == 0
        {
            return;
        }

        if ctx
            .slot_paths
            .get(item_idx)
            .and_then(|path| path.live_path())
            .is_none()
        {
            return;
        }

        let dist_weight = if is_prefetch {
            VIEWPORT_DIST_WEIGHT_PREFETCH
        } else {
            VIEWPORT_DIST_WEIGHT_VISIBLE
        };
        let dist = if dist_weight != 0 {
            let dist_px = tile_center_distance_px(
                ctx,
                TileDistanceInput {
                    obj_x,
                    obj_y,
                    obj_w,
                    obj_h,
                    tiles_x: total_tiles_x,
                    tiles_y: total_tiles_y,
                    tx,
                    ty,
                },
            );
            dist_px.round().min(i32::MAX as f32) as i32
        } else {
            0
        };
        let tiebreak = if is_prefetch {
            0
        } else {
            tile_tiebreak_bias(tx, ty)
        };
        let detail_boost = (16i32 - lod as i32).max(0) * 400;
        let max_penalty = if is_prefetch {
            VIEWPORT_DIST_PREFETCH_MAX_PENALTY
        } else {
            VIEWPORT_DIST_VISIBLE_MAX_PENALTY
        };
        let dist_penalty = viewport_dist_penalty(dist, dist_weight, max_penalty);
        let prio = base_prio + detail_boost - dist_penalty + tiebreak;

        ctx.streaming_runtime
            .canvas_media_slots
            .pending
            .insert(tile_id, ctx.streaming_runtime.stream_epoch);
        let queued = enqueue_canvas_media_slot_request(
            ctx,
            CanvasMediaSlotQueueItem {
                id,
                asset_key,
                item_idx,
                lod,
                x: tx,
                y: ty,
                prio,
                is_prefetch,
                epoch: ctx.streaming_runtime.stream_epoch,
                queued_frame: ctx.frame_count,
            },
        );
        if !queued
            && ctx
                .streaming_runtime
                .canvas_media_slots
                .pending
                .get(&tile_id)
                .copied()
                == Some(ctx.streaming_runtime.stream_epoch)
        {
            ctx.streaming_runtime
                .canvas_media_slots
                .pending
                .remove(&tile_id);
        }
    };

    for ty in vis_min_ty..vis_max_ty {
        for tx in vis_min_tx..vis_max_tx {
            request_canvas_media_slot(tx, ty, base_visible, false);
        }
    }

    for ty in ex_min_ty..ex_max_ty {
        for tx in ex_min_tx..ex_max_tx {
            let inside_visible =
                tx >= vis_min_tx && tx < vis_max_tx && ty >= vis_min_ty && ty < vis_max_ty;
            if inside_visible {
                continue;
            }
            request_canvas_media_slot(tx, ty, base_prefetch, true);
        }
    }
}

pub fn schedule_visible_canvas_media_slots_for_lod(
    ctx: &mut RenderContext,
    id: u64,
    item_idx: usize,
    lod: u8,
    tiles: &math::VisibleTiles,
    base_visible: i32,
) {
    let max_len = ctx.streaming.max_canvas_media_slot_queue_len.max(1);
    let total_len = ctx.streaming_runtime.canvas_media_slots.queue_visible.len()
        + ctx
            .streaming_runtime
            .canvas_media_slots
            .queue_prefetch
            .len();
    if total_len >= max_len
        && ctx
            .streaming_runtime
            .canvas_media_slots
            .queue_prefetch
            .is_empty()
    {
        return;
    }

    let asset_key = ctx.scene.asset_keys.get(item_idx).copied().unwrap_or(0);
    if !crate::core::loader::mem_cache::is_ram_media_slot_asset(asset_key) {
        return;
    }
    let Some((obj_x, obj_y, obj_w, obj_h)) = media_world_geometry(ctx, item_idx) else {
        return;
    };
    let (orig_w, orig_h) = ctx
        .scene
        .item_dimensions
        .get(item_idx)
        .copied()
        .unwrap_or((0, 0));
    let (_, _, total_tiles_x_raw, total_tiles_y_raw, _, _) = lod_info(orig_w, orig_h, lod);
    let Some(region) = ctx.page_directory.get_region(asset_key, lod) else {
        return;
    };
    let total_tiles_x = total_tiles_x_raw.min(region.w);
    let total_tiles_y = total_tiles_y_raw.min(region.h);
    if total_tiles_x == 0 || total_tiles_y == 0 {
        return;
    }
    let vis_min_tx = tiles.min_tx.min(total_tiles_x);
    let vis_max_tx = tiles.max_tx.min(total_tiles_x);
    let vis_min_ty = tiles.min_ty.min(total_tiles_y);
    let vis_max_ty = tiles.max_ty.min(total_tiles_y);
    if vis_min_tx >= vis_max_tx || vis_min_ty >= vis_max_ty {
        return;
    }

    let mut request_canvas_media_slot = |tx: u32, ty: u32, base_prio: i32| {
        let tile_id = CanvasMediaSlotId {
            asset_key,
            lod,
            x: tx,
            y: ty,
        };

        if ctx.page_table.get_slot(tile_id).is_some() {
            ctx.page_table.touch(tile_id);
            return;
        }

        if ctx
            .streaming_runtime
            .canvas_media_slots
            .pending
            .get(&tile_id)
            .copied()
            == Some(ctx.streaming_runtime.stream_epoch)
        {
            return;
        }

        if ctx
            .slot_paths
            .get(item_idx)
            .and_then(|path| path.live_path())
            .is_none()
        {
            return;
        }

        let dist = if VIEWPORT_DIST_WEIGHT_VISIBLE != 0 {
            let dist_px = tile_center_distance_px(
                ctx,
                TileDistanceInput {
                    obj_x,
                    obj_y,
                    obj_w,
                    obj_h,
                    tiles_x: total_tiles_x,
                    tiles_y: total_tiles_y,
                    tx,
                    ty,
                },
            );
            dist_px.round().min(i32::MAX as f32) as i32
        } else {
            0
        };
        let tiebreak = tile_tiebreak_bias(tx, ty);
        let detail_boost = (16i32 - lod as i32).max(0) * 400;
        let dist_penalty = viewport_dist_penalty(
            dist,
            VIEWPORT_DIST_WEIGHT_VISIBLE,
            VIEWPORT_DIST_VISIBLE_MAX_PENALTY,
        );
        let prio = base_prio + detail_boost - dist_penalty + tiebreak;

        ctx.streaming_runtime
            .canvas_media_slots
            .pending
            .insert(tile_id, ctx.streaming_runtime.stream_epoch);
        let queued = enqueue_canvas_media_slot_request(
            ctx,
            CanvasMediaSlotQueueItem {
                id,
                asset_key,
                item_idx,
                lod,
                x: tx,
                y: ty,
                prio,
                is_prefetch: false,
                epoch: ctx.streaming_runtime.stream_epoch,
                queued_frame: ctx.frame_count,
            },
        );
        if !queued
            && ctx
                .streaming_runtime
                .canvas_media_slots
                .pending
                .get(&tile_id)
                .copied()
                == Some(ctx.streaming_runtime.stream_epoch)
        {
            ctx.streaming_runtime
                .canvas_media_slots
                .pending
                .remove(&tile_id);
        }
    };

    for ty in vis_min_ty..vis_max_ty {
        for tx in vis_min_tx..vis_max_tx {
            request_canvas_media_slot(tx, ty, base_visible);
        }
    }
}

pub fn touch_visible_canvas_media_slots_for_lod(
    ctx: &mut RenderContext,
    asset_key: u64,
    lod: u8,
    tiles: &math::VisibleTiles,
) {
    if !crate::core::loader::mem_cache::is_ram_media_slot_asset(asset_key) {
        return;
    }

    let tiles_x = tiles.max_tx.saturating_sub(tiles.min_tx);
    let tiles_y = tiles.max_ty.saturating_sub(tiles.min_ty);
    let total = tiles_x.saturating_mul(tiles_y);
    if total == 0 {
        return;
    }

    const MAX_TOUCH_TILES: u32 = 1024;
    let stride = if total <= MAX_TOUCH_TILES {
        1
    } else {
        ((total as f32 / MAX_TOUCH_TILES as f32).sqrt().ceil() as u32).max(1)
    };
    let step = stride as usize;

    for ty in (tiles.min_ty..tiles.max_ty).step_by(step) {
        for tx in (tiles.min_tx..tiles.max_tx).step_by(step) {
            let tile_id = CanvasMediaSlotId {
                asset_key,
                lod,
                x: tx,
                y: ty,
            };
            if ctx.page_table.get_slot(tile_id).is_some() {
                ctx.page_table.touch(tile_id);
            }
        }
    }
}
