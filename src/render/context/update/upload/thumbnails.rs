use std::time::Instant;

use crate::core::loader::{ImagePayload, LoadedImage};
use crate::render::atlas::{ThumbTier, ThumbnailUploadInput};
use crate::render::context::state::RenderContext;
use crate::render::streaming::preview::thumb_request_key;

const PREVIEW_MISSING_RETRY_FRAMES: u64 = 120;

pub fn upload_thumbnail(ctx: &mut RenderContext, img: LoadedImage) {
    let tier = ThumbTier::from_page_size(img.width).unwrap_or(ThumbTier::Px512);
    let key = thumb_request_key(img.id, tier);
    if let Some(state) = ctx.streaming_runtime.preview.tier_state.get_mut(&img.id) {
        if state.pending == Some(tier) {
            state.pending = None;
        }
    }

    if !ctx.streaming_runtime.slot_interaction_gate.enabled {
        if matches!(
            ctx.streaming_runtime.preview.pending_slots.get(&key).copied(),
            Some(pending) if pending.epoch == img.epoch
        ) {
            ctx.streaming_runtime.preview.pending_slots.remove(&key);
        }
        return;
    }

    if !crate::core::loader::mem_cache::is_ram_media_slot_asset(img.asset_key) {
        if matches!(
            ctx.streaming_runtime.preview.pending_slots.get(&key).copied(),
            Some(pending) if pending.epoch == img.epoch
        ) {
            ctx.streaming_runtime.preview.pending_slots.remove(&key);
        }
        return;
    }

    let pending = ctx
        .streaming_runtime
        .preview
        .pending_slots
        .get(&key)
        .copied();

    if img.epoch != ctx.streaming_runtime.stream_epoch {
        ctx.quality_stats.record_preview_upload_drop_epoch();
        if matches!(pending, Some(p) if p.epoch == img.epoch) {
            ctx.streaming_runtime.preview.pending_slots.remove(&key);
        }
        return;
    }

    let Some(pending) = pending.filter(|p| p.epoch == img.epoch) else {
        // Stale/irrelevant ready thumb: request is no longer tracked for this epoch.
        ctx.quality_stats.record_preview_upload_drop_not_pending();
        return;
    };
    let class = pending.class;
    ctx.streaming_runtime.preview.pending_slots.remove(&key);

    if img.missing {
        ctx.streaming_runtime.preview.retry_after.insert(
            key,
            Instant::now()
                + RenderContext::duration_for_reference_frames(PREVIEW_MISSING_RETRY_FRAMES),
        );
        ctx.quality_stats.record_preview_upload_drop_missing();
        return;
    }
    ctx.streaming_runtime.preview.retry_after.remove(&key);
    if let Some(idx) = ctx.scene.index_for_id(img.id) {
        if idx < ctx.scene.item_dimensions.len() && img.orig_w > 0 && img.orig_h > 0 {
            let next_dims = (img.orig_w, img.orig_h);
            if ctx.scene.item_dimensions[idx] != next_dims {
                ctx.scene.set_item_dimensions(idx, next_dims);
            }
        }
    }

    let ImagePayload::Rgba8(rgba) = img.payload;

    // Upload into the proper tier atlas
    let res = ctx.atlas.upload_thumbnail(ThumbnailUploadInput {
        tier,
        queue: &ctx.gpu.queue,
        id: img.id,
        data: &rgba,
        frame: ctx.frame_count,
        class,
        visible_ids: &ctx.committed_view.visible_ids,
    });

    let Some(res) = res else {
        ctx.quality_stats.record_preview_upload_drop_no_slot();
        return;
    };

    // If someone was evicted in THIS tier, fall back to another tier if possible.
    if let Some(e) = res.evicted {
        ctx.quality_stats.record_preview_eviction();
        if let Some(idx) = ctx.scene.index_for_id(e) {
            if idx < ctx.scene.all_items_raw.len() {
                let uv0 = ctx.scene.all_items_raw[idx].uv_region[0];
                if ThumbTier::decode_uv_x(uv0) == Some(tier) {
                    if let Some((_t, uv)) = ctx.atlas.best_available_uv(e) {
                        let fallback_tier = ThumbTier::decode_uv_x(uv[0]);
                        ctx.scene.update_item_texture(e, uv);
                        if let Some(state) = ctx.streaming_runtime.preview.tier_state.get_mut(&e) {
                            state.display = fallback_tier;
                        }
                    } else {
                        ctx.scene.reset_item_texture(e);
                        if let Some(state) = ctx.streaming_runtime.preview.tier_state.get_mut(&e) {
                            state.display = None;
                        }
                    }
                }
            }
        }
    }

    ctx.scene.update_item_texture(img.id, res.uv_region);
    ctx.streaming_runtime
        .preview
        .tier_state
        .entry(img.id)
        .and_modify(|state| {
            state.display = Some(tier);
            if state.pending == Some(tier) {
                state.pending = None;
            }
        })
        .or_insert(crate::render::context::state::PreviewTierState {
            target: tier,
            display: Some(tier),
            pending: None,
        });
    ctx.quality_stats.record_preview_upload_applied(class);
}
