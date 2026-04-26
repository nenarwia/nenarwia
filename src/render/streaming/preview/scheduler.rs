use crate::core::loader::{LoadRequest, ThumbDecodeMode};
use crate::render::atlas::{ThumbClass, ThumbTier};
use crate::render::context::state::{
    PendingThumbRequest, PreviewMotionTier, PreviewTierState, RenderContext,
};

use super::{pending_preview_cap, thumb_request_key, ThumbRequestKey};

const THUMB_PRIORITY_BOOST_COVERAGE: i32 = 5_000_000;
const THUMB_PRIORITY_BOOST_QUALITY: i32 = 300_000;
const THUMB_PRIORITY_TILE_PRESSURE_PENALTY: i32 = 400_000;

#[inline]
fn is_pending_current(ctx: &RenderContext, key: ThumbRequestKey) -> bool {
    matches!(
        ctx.streaming_runtime.preview.pending_slots.get(&key),
        Some(pending) if pending.epoch == ctx.streaming_runtime.stream_epoch
    )
}

fn drop_one_pending_quality_to_make_room(ctx: &mut RenderContext) -> bool {
    let epoch = ctx.streaming_runtime.stream_epoch;
    let key_to_drop =
        ctx.streaming_runtime
            .preview
            .pending_slots
            .iter()
            .find_map(|(key, pending)| {
                if pending.epoch == epoch && pending.class == ThumbClass::Quality {
                    Some(*key)
                } else {
                    None
                }
            });
    if let Some(key) = key_to_drop {
        ctx.streaming_runtime.preview.pending_slots.remove(&key);
        return true;
    }
    false
}

fn consume_thumb_budget(ctx: &mut RenderContext, class: ThumbClass) -> bool {
    ctx.streaming_runtime.consume_thumb_budget(class)
}

fn coverage_decode_mode(
    ctx: &RenderContext,
    coverage_tier: ThumbTier,
    desired_tier: ThumbTier,
) -> ThumbDecodeMode {
    if (desired_tier as u8) <= (coverage_tier as u8) {
        // If this tier is also the final desired tier, avoid draft/medium stickiness.
        ThumbDecodeMode::Full
    } else {
        match ctx.viewport_runtime().preview_motion_tier {
            PreviewMotionTier::Fast => ThumbDecodeMode::Draft,
            PreviewMotionTier::Medium => ThumbDecodeMode::Medium,
            PreviewMotionTier::Slow => ThumbDecodeMode::Full,
        }
    }
}

pub(crate) fn request_thumbnail_if_needed(
    ctx: &mut RenderContext,
    id: u64,
    item_idx: usize,
    coverage_prio: i32,
    quality_prio: i32,
    allow_quality_requests: bool,
    coverage_tier: ThumbTier,
    desired_tier: ThumbTier,
) {
    let asset_key = ctx.scene.asset_keys.get(item_idx).copied().unwrap_or(0);
    if !crate::core::loader::mem_cache::is_ram_media_slot_asset(asset_key) {
        let epoch = ctx.streaming_runtime.stream_epoch;
        ctx.streaming_runtime
            .preview
            .pending_slots
            .retain(|_, pending| !(pending.epoch == epoch && pending.asset_key == asset_key));
        return;
    }

    let default_target = if allow_quality_requests {
        desired_tier
    } else {
        coverage_tier
    };
    let mut cur_tier = {
        let raw = match ctx.scene.all_items_raw.get(item_idx) {
            Some(r) => r,
            None => return,
        };
        if raw.uv_region[2] > 0.0 {
            ThumbTier::decode_uv_x(raw.uv_region[0])
        } else {
            None
        }
    };
    let mut tier_state = ctx
        .streaming_runtime
        .preview
        .tier_state
        .get(&id)
        .copied()
        .unwrap_or(PreviewTierState {
            target: default_target,
            display: cur_tier,
            pending: None,
        });
    tier_state.display = cur_tier;

    if cur_tier.is_none() {
        if let Some(uv) = ctx.atlas.uv_for_id_tier(id, tier_state.target) {
            ctx.scene.update_item_texture(id, uv);
            cur_tier = Some(tier_state.target);
        } else if let Some((fallback_tier, uv)) = ctx.atlas.best_available_uv(id) {
            ctx.scene.update_item_texture(id, uv);
            cur_tier = Some(fallback_tier);
        }
        tier_state.display = cur_tier;
    }

    let mut missing_current = false;

    if let Some(t) = cur_tier {
        ctx.atlas.touch(t, id, ctx.frame_count);
        if !ctx.atlas.has(t, id) {
            if let Some(uv) = ctx.atlas.uv_for_id_tier(id, tier_state.target) {
                ctx.scene.update_item_texture(id, uv);
                cur_tier = Some(tier_state.target);
            } else if let Some((fallback_tier, uv)) = ctx.atlas.best_available_uv(id) {
                ctx.scene.update_item_texture(id, uv);
                cur_tier = Some(fallback_tier);
            } else {
                ctx.scene.reset_item_texture(id);
                cur_tier = None;
                missing_current = true;
            }
            tier_state.display = cur_tier;
        }
    }

    let coverage_ready = match cur_tier {
        None => false,
        Some(t) => (t as u8) >= (coverage_tier as u8),
    };

    if !coverage_ready || missing_current {
        let enqueued = enqueue_thumb_request(
            ctx,
            id,
            item_idx,
            coverage_prio,
            coverage_tier,
            ThumbClass::Coverage,
            coverage_decode_mode(ctx, coverage_tier, desired_tier),
        );
        if enqueued {
            tier_state.pending = Some(coverage_tier);
        }
    }

    if !allow_quality_requests {
        ctx.streaming_runtime
            .preview
            .tier_state
            .insert(id, tier_state);
        return;
    }

    tier_state.target = desired_tier;
    if let Some(uv) = ctx.atlas.uv_for_id_tier(id, tier_state.target) {
        if tier_state.display != Some(tier_state.target) {
            ctx.scene.update_item_texture(id, uv);
            tier_state.display = Some(tier_state.target);
        }
        if tier_state.pending == Some(tier_state.target) {
            tier_state.pending = None;
        }
        ctx.streaming_runtime
            .preview
            .tier_state
            .insert(id, tier_state);
        return;
    }

    if tier_state.display == Some(tier_state.target) && !missing_current {
        ctx.streaming_runtime
            .preview
            .tier_state
            .insert(id, tier_state);
        return;
    }

    let target_class = if (tier_state.target as u8) <= (coverage_tier as u8) {
        ThumbClass::Coverage
    } else {
        ThumbClass::Quality
    };
    let target_prio = if matches!(target_class, ThumbClass::Coverage) {
        coverage_prio
    } else {
        quality_prio
    };
    let target_decode_mode = if matches!(target_class, ThumbClass::Coverage) {
        coverage_decode_mode(ctx, coverage_tier, desired_tier)
    } else {
        ThumbDecodeMode::Full
    };

    let enqueued = enqueue_thumb_request(
        ctx,
        id,
        item_idx,
        target_prio,
        tier_state.target,
        target_class,
        target_decode_mode,
    );
    if enqueued {
        tier_state.pending = Some(tier_state.target);
    }
    ctx.streaming_runtime
        .preview
        .tier_state
        .insert(id, tier_state);
}

fn enqueue_thumb_request(
    ctx: &mut RenderContext,
    id: u64,
    item_idx: usize,
    prio: i32,
    tier: ThumbTier,
    class: ThumbClass,
    decode_mode: ThumbDecodeMode,
) -> bool {
    let key = thumb_request_key(id, tier);

    let now = std::time::Instant::now();
    if let Some(retry_after_at) = ctx.streaming_runtime.preview.retry_after.get(&key).copied() {
        if retry_after_at > now {
            return false;
        }
        ctx.streaming_runtime.preview.retry_after.remove(&key);
    }

    if is_pending_current(ctx, key) {
        if class == ThumbClass::Coverage {
            if let Some(pending) = ctx.streaming_runtime.preview.pending_slots.get_mut(&key) {
                pending.class = ThumbClass::Coverage;
            }
        }
        return true;
    }

    let pending_cap = pending_preview_cap(ctx);
    if ctx.streaming_runtime.preview.pending_slots.len() >= pending_cap {
        if class == ThumbClass::Coverage {
            if !drop_one_pending_quality_to_make_room(ctx)
                && ctx.streaming_runtime.preview.pending_slots.len() >= pending_cap
            {
                return false;
            }
        } else {
            return false;
        }
    }

    let Some(path) = ctx
        .slot_paths
        .get(item_idx)
        .and_then(|path| path.live_path())
        .map(|path| path.to_path_buf())
    else {
        return false;
    };
    let (orig_w, orig_h) = ctx
        .scene
        .item_dimensions
        .get(item_idx)
        .copied()
        .unwrap_or((0, 0));
    let asset_key = ctx.scene.asset_keys.get(item_idx).copied().unwrap_or(0);

    if !consume_thumb_budget(ctx, class) {
        return false;
    }

    ctx.streaming_runtime.preview.pending_slots.insert(
        key,
        PendingThumbRequest {
            epoch: ctx.streaming_runtime.stream_epoch,
            class,
            asset_key,
            tier: tier.page_size() as u16,
        },
    );

    let mut prio = prio.saturating_add(match class {
        ThumbClass::Coverage => THUMB_PRIORITY_BOOST_COVERAGE,
        ThumbClass::Quality => THUMB_PRIORITY_BOOST_QUALITY,
    });

    // Keep first preview fast, but let visible tile-quality jobs win while they are pending.
    if matches!(class, ThumbClass::Quality)
        && (ctx.has_pending_canvas_media_slots_current()
            || !ctx
                .streaming_runtime
                .canvas_media_slots
                .queue_visible
                .is_empty())
    {
        prio = prio.saturating_sub(THUMB_PRIORITY_TILE_PRESSURE_PENALTY);
    }

    let _ = ctx.loader.request_prio(
        LoadRequest::Thumbnail {
            path,
            id,
            asset_key,
            size: tier.page_size() as u16,
            decode_mode,
            epoch: ctx.streaming_runtime.stream_epoch,
            orig_w,
            orig_h,
        },
        prio,
    );
    true
}
