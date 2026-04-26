use std::collections::{BTreeSet, BinaryHeap};

use crate::core::loader::LoadRequest;
use crate::core::metrics;
use crate::render::cache::{math, CanvasMediaSlotId};
use crate::render::context::state::RenderContext;

use super::super::calculator::{lod_info, media_world_geometry};
use super::super::CanvasMediaSlotQueueItem;

const VISIBLE_STARVATION_BOOST_PER_FRAME: i32 = 200;
const VISIBLE_STARVATION_BOOST_MAX_FRAMES: u64 = 180;
const PREFETCH_STARVATION_BOOST_PER_FRAME: i32 = 80;
const PREFETCH_STARVATION_BOOST_MAX_FRAMES: u64 = 240;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CanvasMediaSlotRelevance {
    Visible,
    PrefetchOnly,
    Stale,
}

fn classify_canvas_media_slot_relevance_now(
    ctx: &RenderContext,
    item: &CanvasMediaSlotQueueItem,
) -> CanvasMediaSlotRelevance {
    let (orig_w, orig_h) = ctx
        .scene
        .item_dimensions
        .get(item.item_idx)
        .copied()
        .unwrap_or((0, 0));
    if orig_w == 0 || orig_h == 0 {
        return CanvasMediaSlotRelevance::Stale;
    }

    let (lod_w, lod_h, tiles_x, tiles_y, _, _) = lod_info(orig_w, orig_h, item.lod);
    if tiles_x == 0 || tiles_y == 0 {
        return CanvasMediaSlotRelevance::Stale;
    }
    let Some((obj_x, obj_y, obj_w, obj_h)) = media_world_geometry(ctx, item.item_idx) else {
        return CanvasMediaSlotRelevance::Stale;
    };

    let view = ctx.view().viewport_rect();
    let Some(vis) =
        math::calculate_visible_tiles_f64(view, obj_x, obj_y, obj_w, obj_h, lod_w, lod_h)
    else {
        return CanvasMediaSlotRelevance::Stale;
    };

    if item.x >= vis.min_tx && item.x < vis.max_tx && item.y >= vis.min_ty && item.y < vis.max_ty {
        return CanvasMediaSlotRelevance::Visible;
    }

    let r = if ctx.viewport_runtime().moving_recently {
        0
    } else {
        ctx.streaming.prefetch_radius_tiles
    };
    let ex_min_tx = vis.min_tx.saturating_sub(r);
    let ex_max_tx = vis.max_tx.saturating_add(r).min(tiles_x);
    let ex_min_ty = vis.min_ty.saturating_sub(r);
    let ex_max_ty = vis.max_ty.saturating_add(r).min(tiles_y);
    if item.x >= ex_min_tx && item.x < ex_max_tx && item.y >= ex_min_ty && item.y < ex_max_ty {
        CanvasMediaSlotRelevance::PrefetchOnly
    } else {
        CanvasMediaSlotRelevance::Stale
    }
}

pub fn drain_canvas_media_slot_queue(ctx: &mut RenderContext) {
    if !ctx.streaming_runtime.slot_interaction_gate.enabled {
        metrics::set_tiles_started_per_frame(0);
        return;
    }

    let mut remaining = ctx
        .streaming_runtime
        .budgets
        .canvas_media_slot_budget_remaining;
    if remaining == 0 {
        metrics::set_tiles_started_per_frame(0);
        return;
    }

    let mut visible = std::mem::take(&mut ctx.streaming_runtime.canvas_media_slots.queue_visible);
    let mut prefetch = std::mem::take(&mut ctx.streaming_runtime.canvas_media_slots.queue_prefetch);

    let mut started_this_frame = 0usize;
    let mut visible_started_ignoring_cpu_budget = 0usize;
    let mut budget_hit = false;
    let mut min_visible = ctx
        .streaming_runtime
        .budgets
        .canvas_media_slot_min_visible_remaining;
    drain_visible_queue(
        ctx,
        &mut visible,
        &mut remaining,
        &mut started_this_frame,
        &mut budget_hit,
        &mut min_visible,
        &mut visible_started_ignoring_cpu_budget,
        true,
    );
    if remaining > 0 {
        drain_prefetch_queue(
            ctx,
            &mut prefetch,
            &mut remaining,
            &mut started_this_frame,
            &mut budget_hit,
            true,
        );
    }

    ctx.streaming_runtime.canvas_media_slots.queue_visible = visible;
    ctx.streaming_runtime.canvas_media_slots.queue_prefetch = prefetch;

    ctx.streaming_runtime
        .consume_canvas_media_slot_budget(started_this_frame);
    ctx.streaming_runtime
        .consume_canvas_media_slot_min_visible_budget(visible_started_ignoring_cpu_budget);
    metrics::set_tiles_started_per_frame(started_this_frame as u64);
    if budget_hit {
        metrics::record_frame_budget_hit();
    }
}

fn drain_visible_queue(
    ctx: &mut RenderContext,
    queue: &mut BinaryHeap<CanvasMediaSlotQueueItem>,
    remaining: &mut usize,
    started_this_frame: &mut usize,
    budget_hit: &mut bool,
    min_visible: &mut usize,
    visible_started_ignoring_cpu_budget: &mut usize,
    enforce_budget: bool,
) {
    while *remaining > 0 {
        let Some(item) = queue.pop() else {
            break;
        };
        let tile_id = CanvasMediaSlotId {
            asset_key: item.asset_key,
            lod: item.lod,
            x: item.x,
            y: item.y,
        };

        if item.epoch != ctx.streaming_runtime.stream_epoch {
            if ctx
                .streaming_runtime
                .canvas_media_slots
                .pending
                .get(&tile_id)
                .copied()
                == Some(item.epoch)
            {
                ctx.streaming_runtime
                    .canvas_media_slots
                    .pending
                    .remove(&tile_id);
            }
            continue;
        }

        if ctx
            .streaming_runtime
            .canvas_media_slots
            .pending
            .get(&tile_id)
            .copied()
            != Some(item.epoch)
        {
            continue;
        }

        if !crate::core::loader::mem_cache::is_ram_media_slot_asset(item.asset_key) {
            if ctx
                .streaming_runtime
                .canvas_media_slots
                .pending
                .get(&tile_id)
                .copied()
                == Some(item.epoch)
            {
                ctx.streaming_runtime
                    .canvas_media_slots
                    .pending
                    .remove(&tile_id);
            }
            continue;
        }

        if classify_canvas_media_slot_relevance_now(ctx, &item) != CanvasMediaSlotRelevance::Visible
        {
            if ctx
                .streaming_runtime
                .canvas_media_slots
                .pending
                .get(&tile_id)
                .copied()
                == Some(item.epoch)
            {
                ctx.streaming_runtime
                    .canvas_media_slots
                    .pending
                    .remove(&tile_id);
            }
            metrics::record_tile_pruned_stale(false);
            continue;
        }

        let Some(path) = ctx
            .slot_paths
            .get(item.item_idx)
            .and_then(|path| path.live_path())
        else {
            if ctx
                .streaming_runtime
                .canvas_media_slots
                .pending
                .get(&tile_id)
                .copied()
                == Some(item.epoch)
            {
                ctx.streaming_runtime
                    .canvas_media_slots
                    .pending
                    .remove(&tile_id);
            }
            continue;
        };
        let (orig_w, orig_h) = ctx
            .scene
            .item_dimensions
            .get(item.item_idx)
            .copied()
            .unwrap_or((0, 0));

        let inflight = ctx.loader.inflight_canvas_media_slots();
        if inflight >= ctx.streaming.max_inflight_canvas_media_slots {
            *budget_hit = true;
            queue.push(item);
            break;
        }

        if enforce_budget {
            let mut ignore_budget = false;
            if *min_visible > 0 {
                ignore_budget = true;
                *min_visible = min_visible.saturating_sub(1);
                *visible_started_ignoring_cpu_budget =
                    visible_started_ignoring_cpu_budget.saturating_add(1);
            }
            if let Some(budget) = ctx
                .streaming_runtime
                .budgets
                .canvas_media_slot_cpu_budget_for_update
            {
                let avg_ms = metrics::avg_tile_build_ms().max(0.25);
                let projected = avg_ms * (*started_this_frame as f32 + 1.0);
                let budget_ms = budget.as_secs_f32() * 1000.0;
                if !ignore_budget && projected > budget_ms {
                    *budget_hit = true;
                    queue.push(item);
                    break;
                }
            }
        }

        let wait_frames_raw = ctx.frame_count.saturating_sub(item.queued_frame);
        let wait_frames = wait_frames_raw.min(VISIBLE_STARVATION_BOOST_MAX_FRAMES);
        let wait_boost = (wait_frames as i32).saturating_mul(VISIBLE_STARVATION_BOOST_PER_FRAME);
        let dispatch_prio = item.prio.saturating_add(wait_boost);
        let offscreen = false;

        let _ = ctx.loader.request_prio(
            LoadRequest::CanvasMediaSlot {
                path: path.to_path_buf(),
                id: item.id,
                asset_key: item.asset_key,
                lod: item.lod,
                tile_x: item.x,
                tile_y: item.y,
                epoch: item.epoch,
                orig_w,
                orig_h,
            },
            dispatch_prio,
        );
        metrics::record_tile_dispatched(false, offscreen, wait_frames_raw);
        *remaining = remaining.saturating_sub(1);
        *started_this_frame = started_this_frame.saturating_add(1);
    }
}

fn drain_prefetch_queue(
    ctx: &mut RenderContext,
    queue: &mut BTreeSet<CanvasMediaSlotQueueItem>,
    remaining: &mut usize,
    started_this_frame: &mut usize,
    budget_hit: &mut bool,
    enforce_budget: bool,
) {
    while *remaining > 0 {
        let Some(item) = queue.pop_last() else {
            break;
        };
        let tile_id = CanvasMediaSlotId {
            asset_key: item.asset_key,
            lod: item.lod,
            x: item.x,
            y: item.y,
        };

        if item.epoch != ctx.streaming_runtime.stream_epoch {
            if ctx
                .streaming_runtime
                .canvas_media_slots
                .pending
                .get(&tile_id)
                .copied()
                == Some(item.epoch)
            {
                ctx.streaming_runtime
                    .canvas_media_slots
                    .pending
                    .remove(&tile_id);
            }
            continue;
        }

        if ctx
            .streaming_runtime
            .canvas_media_slots
            .pending
            .get(&tile_id)
            .copied()
            != Some(item.epoch)
        {
            continue;
        }

        if !crate::core::loader::mem_cache::is_ram_media_slot_asset(item.asset_key) {
            if ctx
                .streaming_runtime
                .canvas_media_slots
                .pending
                .get(&tile_id)
                .copied()
                == Some(item.epoch)
            {
                ctx.streaming_runtime
                    .canvas_media_slots
                    .pending
                    .remove(&tile_id);
            }
            continue;
        }

        if classify_canvas_media_slot_relevance_now(ctx, &item) == CanvasMediaSlotRelevance::Stale {
            if ctx
                .streaming_runtime
                .canvas_media_slots
                .pending
                .get(&tile_id)
                .copied()
                == Some(item.epoch)
            {
                ctx.streaming_runtime
                    .canvas_media_slots
                    .pending
                    .remove(&tile_id);
            }
            metrics::record_tile_pruned_stale(true);
            continue;
        }

        let Some(path) = ctx
            .slot_paths
            .get(item.item_idx)
            .and_then(|path| path.live_path())
        else {
            if ctx
                .streaming_runtime
                .canvas_media_slots
                .pending
                .get(&tile_id)
                .copied()
                == Some(item.epoch)
            {
                ctx.streaming_runtime
                    .canvas_media_slots
                    .pending
                    .remove(&tile_id);
            }
            continue;
        };
        let (orig_w, orig_h) = ctx
            .scene
            .item_dimensions
            .get(item.item_idx)
            .copied()
            .unwrap_or((0, 0));

        let inflight = ctx.loader.inflight_canvas_media_slots();
        if inflight >= ctx.streaming.max_inflight_canvas_media_slots {
            *budget_hit = true;
            queue.insert(item);
            break;
        }

        if enforce_budget {
            if let Some(budget) = ctx
                .streaming_runtime
                .budgets
                .canvas_media_slot_cpu_budget_for_update
            {
                let avg_ms = metrics::avg_tile_build_ms().max(0.25);
                let projected = avg_ms * (*started_this_frame as f32 + 1.0);
                let budget_ms = budget.as_secs_f32() * 1000.0;
                if projected > budget_ms {
                    *budget_hit = true;
                    queue.insert(item);
                    break;
                }
            }
        }

        let wait_frames_raw = ctx.frame_count.saturating_sub(item.queued_frame);
        let wait_frames = wait_frames_raw.min(PREFETCH_STARVATION_BOOST_MAX_FRAMES);
        let wait_boost = (wait_frames as i32).saturating_mul(PREFETCH_STARVATION_BOOST_PER_FRAME);
        let dispatch_prio = item.prio.saturating_add(wait_boost);
        let offscreen = false;

        let _ = ctx.loader.request_prio(
            LoadRequest::CanvasMediaSlot {
                path: path.to_path_buf(),
                id: item.id,
                asset_key: item.asset_key,
                lod: item.lod,
                tile_x: item.x,
                tile_y: item.y,
                epoch: item.epoch,
                orig_w,
                orig_h,
            },
            dispatch_prio,
        );
        metrics::record_tile_dispatched(true, offscreen, wait_frames_raw);
        *remaining = remaining.saturating_sub(1);
        *started_this_frame = started_this_frame.saturating_add(1);
    }
}

pub fn enqueue_canvas_media_slot_request(
    ctx: &mut RenderContext,
    item: CanvasMediaSlotQueueItem,
) -> bool {
    let max_len = ctx.streaming.max_canvas_media_slot_queue_len.max(1);
    let total_len = ctx.streaming_runtime.canvas_media_slots.queue_visible.len()
        + ctx
            .streaming_runtime
            .canvas_media_slots
            .queue_prefetch
            .len();

    if item.is_prefetch {
        if total_len >= max_len {
            metrics::record_tile_enqueue_drop(true);
            return false;
        }
        ctx.streaming_runtime
            .canvas_media_slots
            .queue_prefetch
            .insert(item);
        metrics::record_tile_enqueued(true);
        return true;
    }

    if total_len >= max_len {
        if let Some(dropped) = ctx
            .streaming_runtime
            .canvas_media_slots
            .queue_prefetch
            .pop_first()
        {
            let dropped_id = CanvasMediaSlotId {
                asset_key: dropped.asset_key,
                lod: dropped.lod,
                x: dropped.x,
                y: dropped.y,
            };
            if ctx
                .streaming_runtime
                .canvas_media_slots
                .pending
                .get(&dropped_id)
                .copied()
                == Some(dropped.epoch)
            {
                ctx.streaming_runtime
                    .canvas_media_slots
                    .pending
                    .remove(&dropped_id);
            }
            metrics::record_tile_enqueue_drop(true);
        } else {
            metrics::record_tile_enqueue_drop(false);
            return false;
        }
    }

    ctx.streaming_runtime
        .canvas_media_slots
        .queue_visible
        .push(item);
    metrics::record_tile_enqueued(false);
    true
}
