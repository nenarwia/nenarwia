use std::collections::{BTreeSet, BinaryHeap};

use crate::render::cache::CanvasMediaSlotId;
use crate::render::context::state::RenderContext;

use super::CanvasMediaSlotQueueItem;

pub(super) fn process_evictions(
    ctx: &mut RenderContext,
    evicted: &[(u64, u8, crate::render::cache::directory::PtRegion)],
) {
    if !evicted.is_empty() {
        ctx.quality_stats
            .record_page_dir_evictions(evicted.len() as u32);
    }

    for (victim_asset, victim_lod, _victim_region) in evicted.iter().copied() {
        ctx.page_table
            .invalidate_asset_lod(victim_asset, victim_lod);

        let to_remove: Vec<CanvasMediaSlotId> = ctx
            .streaming_runtime
            .canvas_media_slots
            .pending
            .keys()
            .filter(|tile| tile.asset_key == victim_asset && tile.lod == victim_lod)
            .copied()
            .collect();
        for tile in to_remove {
            ctx.streaming_runtime
                .canvas_media_slots
                .pending
                .remove(&tile);
        }

        retain_queue(
            &mut ctx.streaming_runtime.canvas_media_slots.queue_visible,
            |queued| !(queued.asset_key == victim_asset && queued.lod == victim_lod),
        );
        retain_prefetch_queue(
            &mut ctx.streaming_runtime.canvas_media_slots.queue_prefetch,
            |queued| !(queued.asset_key == victim_asset && queued.lod == victim_lod),
        );

        if let Some(idx) = ctx.scene.index_for_asset(victim_asset) {
            if idx < ctx.scene.all_items_raw.len() {
                let render_matches = ctx.scene.render_lod.get(idx).copied() == Some(victim_lod);
                let coarse_matches = ctx.scene.coarse_lod.get(idx).copied() == Some(victim_lod);

                if render_matches || coarse_matches {
                    {
                        let raw = &mut ctx.scene.all_items_raw[idx];
                        if render_matches {
                            raw.params = [-1.0, -1.0, 0.0, 0.0];
                        }
                        if coarse_matches {
                            raw.params2 = [-1.0, -1.0, 0.0, 0.0];
                        }
                    }

                    if render_matches && idx < ctx.scene.render_lod.len() {
                        ctx.scene.render_lod[idx] = u8::MAX;
                    }
                    if coarse_matches && idx < ctx.scene.coarse_lod.len() {
                        ctx.scene.coarse_lod[idx] = u8::MAX;
                    }
                }
            }
        }
    }
}

fn retain_queue<F>(queue: &mut BinaryHeap<CanvasMediaSlotQueueItem>, keep: F)
where
    F: Fn(&CanvasMediaSlotQueueItem) -> bool,
{
    if queue.is_empty() {
        return;
    }

    let mut items = Vec::with_capacity(queue.len());
    while let Some(item) = queue.pop() {
        if keep(&item) {
            items.push(item);
        }
    }
    *queue = items.into_iter().collect();
}

fn retain_prefetch_queue<F>(queue: &mut BTreeSet<CanvasMediaSlotQueueItem>, keep: F)
where
    F: Fn(&CanvasMediaSlotQueueItem) -> bool,
{
    if queue.is_empty() {
        return;
    }

    let mut items = Vec::with_capacity(queue.len());
    for item in queue.iter().copied() {
        if keep(&item) {
            items.push(item);
        }
    }
    *queue = items.into_iter().collect();
}
