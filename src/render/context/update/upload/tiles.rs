use crate::core::loader::{ImagePayload, LoadedImage};
use crate::render::cache::CanvasMediaSlotId;
use crate::render::context::state::RenderContext;

pub fn upload_tile(ctx: &mut RenderContext, img: LoadedImage) {
    let tile_id = CanvasMediaSlotId {
        asset_key: img.asset_key,
        lod: img.lod,
        x: img.tile_x,
        y: img.tile_y,
    };

    if !ctx.streaming_runtime.slot_interaction_gate.enabled {
        if ctx
            .streaming_runtime
            .canvas_media_slots
            .pending
            .get(&tile_id)
            .copied()
            == Some(img.epoch)
        {
            ctx.streaming_runtime
                .canvas_media_slots
                .pending
                .remove(&tile_id);
        }
        return;
    }

    if img.epoch != ctx.streaming_runtime.stream_epoch {
        if ctx
            .streaming_runtime
            .canvas_media_slots
            .pending
            .get(&tile_id)
            .copied()
            == Some(img.epoch)
        {
            ctx.streaming_runtime
                .canvas_media_slots
                .pending
                .remove(&tile_id);
        }
        return;
    }

    if ctx
        .streaming_runtime
        .canvas_media_slots
        .pending
        .get(&tile_id)
        .copied()
        == Some(img.epoch)
    {
        ctx.streaming_runtime
            .canvas_media_slots
            .pending
            .remove(&tile_id);
    }

    if !crate::core::loader::mem_cache::is_ram_media_slot_asset(img.asset_key) {
        return;
    }

    if let Some(idx) = ctx.scene.index_for_id(img.id) {
        if img.orig_w > 0 && img.orig_h > 0 {
            let dims = (img.orig_w, img.orig_h);
            if ctx.scene.item_dimensions.get(idx).copied() != Some(dims) {
                ctx.scene.set_item_dimensions(idx, dims);
            }
        }
    }

    if img.missing {
        return;
    }

    if img.width == 0 || img.height == 0 {
        return;
    }

    let Some(region_for_tile) = ctx.page_directory.get_region(img.asset_key, img.lod) else {
        return;
    };

    let ImagePayload::Rgba8(upload_bytes) = img.payload;
    if !ctx.tile_cache.payload_len_matches(&upload_bytes) {
        return;
    }

    // 1) Allocate slot in physical cache (may evict).
    let Some((slot, evicted_tile)) = ctx.page_table.allocate(tile_id) else {
        log::warn!(
            "tile allocation failed: asset={} lod={} x={} y={}",
            tile_id.asset_key,
            tile_id.lod,
            tile_id.x,
            tile_id.y
        );
        return;
    };
    if evicted_tile.is_some() {
        ctx.quality_stats.record_tile_eviction();
        ctx.stage0_metrics.record_evicted_pages(1);
    }

    if let Some(victim) = evicted_tile {
        if let Some(region) = ctx.page_directory.get_region(victim.asset_key, victim.lod) {
            ctx.page_directory
                .update_entry(&ctx.gpu.queue, region, victim.x, victim.y, None);
        }
    }

    // 2) Upload tile data.
    ctx.tile_cache
        .upload_tile(&ctx.gpu.queue, slot, &upload_bytes);

    // 3) Update page directory.
    ctx.page_directory.update_entry(
        &ctx.gpu.queue,
        region_for_tile,
        img.tile_x,
        img.tile_y,
        Some(slot),
    );
}
