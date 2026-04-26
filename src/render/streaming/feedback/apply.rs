use std::collections::HashMap;

use crate::render::cache::CanvasMediaSlotId;
use crate::render::context::state::RenderContext;
use crate::render::streaming::canvas_media_slots::calculator::{
    tile_tiebreak_bias, QUALITY_DEBT_BUCKET_SCALE, QUALITY_DEBT_BUCKET_WEIGHT,
};
use crate::render::streaming::canvas_media_slots::{
    enqueue_canvas_media_slot_request, CanvasMediaSlotQueueItem,
};

use super::{FeedbackResult, FeedbackSummary};

pub(super) fn apply_feedback_results_impl(
    ctx: &mut RenderContext,
    results: Vec<FeedbackResult>,
) -> FeedbackSummary {
    if results.is_empty() {
        return FeedbackSummary {
            unique_tiles: 0,
            overflow: false,
            latency_frames: 0,
        };
    }

    let mut overflow = false;
    let mut max_latency = 0u32;
    let mut unique: HashMap<CanvasMediaSlotId, u32> = HashMap::new();

    for result in results {
        overflow |= result.overflow;
        if result.latency_frames > max_latency {
            max_latency = result.latency_frames;
        }
        for tile in result.tiles.iter() {
            let asset_key = (tile.asset_key_hi as u64) << 32 | (tile.asset_key_lo as u64);
            let x = tile.tile_x;
            let y = tile.tile_y;
            let lod = (tile.lod.min(u8::MAX as u32)) as u8;
            let tile_id = CanvasMediaSlotId {
                asset_key,
                lod,
                x,
                y,
            };
            *unique.entry(tile_id).or_insert(0) += 1;
        }
    }

    let unique_len = unique.len() as u32;
    for (tile_id, freq) in unique.iter() {
        if ctx.page_table.get_slot(*tile_id).is_some() {
            ctx.page_table.touch(*tile_id);
            continue;
        }
        if ctx
            .streaming_runtime
            .canvas_media_slots
            .pending
            .get(tile_id)
            .copied()
            == Some(ctx.streaming_runtime.stream_epoch)
        {
            continue;
        }
        let Some(item_idx) = ctx.scene.index_for_asset(tile_id.asset_key) else {
            continue;
        };
        if ctx
            .slot_paths
            .get(item_idx)
            .and_then(|path| path.live_path())
            .is_none()
        {
            continue;
        }

        let id = ctx.scene.index_to_id.get(item_idx).copied().unwrap_or(0);
        let debt = ctx.scene.quality_debt.get(item_idx).copied().unwrap_or(0.0);
        let debt_bucket = (debt * QUALITY_DEBT_BUCKET_SCALE).round().max(0.0) as i32;
        let debt_boost = debt_bucket.saturating_mul(QUALITY_DEBT_BUCKET_WEIGHT);
        let detail_boost = (16i32 - tile_id.lod as i32).max(0) * 400;
        let freq_boost = ((*freq).min(255) as i32) * 10;
        let tiebreak = tile_tiebreak_bias(tile_id.x, tile_id.y);
        let prio = debt_boost + detail_boost + freq_boost + tiebreak;

        ctx.streaming_runtime
            .canvas_media_slots
            .pending
            .insert(*tile_id, ctx.streaming_runtime.stream_epoch);
        let queued = enqueue_canvas_media_slot_request(
            ctx,
            CanvasMediaSlotQueueItem {
                id,
                asset_key: tile_id.asset_key,
                item_idx,
                lod: tile_id.lod,
                x: tile_id.x,
                y: tile_id.y,
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
                .get(tile_id)
                .copied()
                == Some(ctx.streaming_runtime.stream_epoch)
        {
            ctx.streaming_runtime
                .canvas_media_slots
                .pending
                .remove(tile_id);
        }
    }

    FeedbackSummary {
        unique_tiles: unique_len,
        overflow,
        latency_frames: max_latency,
    }
}
