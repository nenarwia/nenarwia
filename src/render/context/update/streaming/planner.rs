use crate::render::atlas::ThumbClass;
use crate::render::context::state::{RenderContext, VisibleItem};
use crate::render::streaming as render_streaming;
use crate::render::streaming::preview::pending_preview_cap;

use super::ordering::{
    build_preview_order_soft_center, build_preview_order_strict_center, visible_item_ring,
};
use super::pending::{
    boost_preview_coverage_budget, desired_preview_coverage_outstanding,
    drop_pending_quality_previews, drop_pending_quality_to_cap, pending_preview_class_counts,
    preview_retry_blocked_at, prune_pending_previews_to_visible, prune_preview_retry_after,
    sync_loader_preview_queue_with_pending, update_preview_ring_lock,
};
use super::preview_checks::{
    coverage_thumb_key_for_item, item_has_known_dimensions, preview_has_any, preview_has_coverage,
};
use super::residency::resident_media_slot_assets_for_frame;
use super::slot_gate::update_slot_interaction_gate;
use super::{PREVIEW_RING_COUNT, PREVIEW_RING_LOCK_IDLE_BAND, PREVIEW_RING_LOCK_MOVING_BAND};

#[derive(Clone, Copy)]
struct RingItem {
    item: VisibleItem,
    ring: usize,
}

#[inline]
fn item_is_ram_media_slot(
    ctx: &RenderContext,
    ram_assets: &std::collections::HashSet<u64>,
    item_idx: usize,
) -> bool {
    let asset_key = ctx.scene.asset_keys.get(item_idx).copied().unwrap_or(0);
    ram_assets.contains(&asset_key)
}

fn enqueue_preview_coverage_if_slot(
    ctx: &mut RenderContext,
    item: VisibleItem,
    coverage_slots: &mut usize,
    pending_coverage: &mut u32,
) {
    if *coverage_slots == 0 {
        return;
    }
    let Some(key) = coverage_thumb_key_for_item(ctx, item.id, item.idx) else {
        return;
    };
    let was_pending = matches!(
        ctx.streaming_runtime.preview.pending_slots.get(&key),
        Some(p) if p.epoch == ctx.streaming_runtime.stream_epoch && p.class == ThumbClass::Coverage
    );
    render_streaming::handle_preview_coverage_request(ctx, item.id, item.idx);
    let now_pending = matches!(
        ctx.streaming_runtime.preview.pending_slots.get(&key),
        Some(p) if p.epoch == ctx.streaming_runtime.stream_epoch && p.class == ThumbClass::Coverage
    );
    if !was_pending && now_pending {
        *coverage_slots = coverage_slots.saturating_sub(1);
        *pending_coverage = pending_coverage.saturating_add(1);
    }
}

fn refresh_quality_visibility_tracking(ctx: &mut RenderContext) {
    let frame = ctx.frame_count;
    for item in ctx.committed_view.visible_items.iter().copied() {
        ctx.quality_visible_since.entry(item.id).or_insert(frame);
    }

    const QUALITY_CLEANUP_FRAMES: u64 = 120;
    if frame.saturating_sub(ctx.quality_last_cleanup_frame) < QUALITY_CLEANUP_FRAMES {
        return;
    }

    let visible_ids = ctx.committed_view.visible_ids.clone();
    ctx.quality_visible_since
        .retain(|id, _| visible_ids.contains(id));
    ctx.quality_last_cleanup_frame = frame;
}

pub(super) fn process_committed_view(ctx: &mut RenderContext) {
    update_slot_interaction_gate(ctx, ctx.committed_view.visible_items.len());
    if !ctx.streaming_runtime.slot_interaction_gate.enabled {
        ctx.committed_view.clear_visible_ids();
        ctx.streaming_runtime.clear_preview_completion_tracking();
        ctx.quality_stats.record_visible_preview_coverage_last(0, 0);
        ctx.quality_stats
            .record_preview_phase_last(0, 0, false, false, 0, 0, 0, 0);
        ctx.quality_stats
            .record_visible_items_last(ctx.committed_view.visible_items.len() as u32, 0);
        return;
    }

    let ram_assets = resident_media_slot_assets_for_frame(ctx);

    let before_ram_filter_pending = ctx.streaming_runtime.preview.pending_slots.len();
    let epoch = ctx.streaming_runtime.stream_epoch;
    ctx.streaming_runtime
        .preview
        .pending_slots
        .retain(|_, pending| pending.epoch == epoch && ram_assets.contains(&pending.asset_key));
    let pending_ram_filtered = before_ram_filter_pending
        .saturating_sub(ctx.streaming_runtime.preview.pending_slots.len())
        as u32;

    ctx.committed_view.rebuild_visible_ids();
    prune_preview_retry_after(ctx);
    let pending_pruned = prune_pending_previews_to_visible(ctx);
    refresh_quality_visibility_tracking(ctx);

    let coverage_items = build_preview_order_strict_center(ctx, &ctx.committed_view.visible_items);
    let mut coverage_missing_any: Vec<RingItem> = Vec::new();
    let mut coverage_upgrade: Vec<RingItem> = Vec::new();
    let mut ring_missing = [0u32; PREVIEW_RING_COUNT];
    let retry_now = std::time::Instant::now();
    for item in coverage_items.iter().copied() {
        if !item_is_ram_media_slot(ctx, &ram_assets, item.idx) {
            continue;
        }
        if !item_has_known_dimensions(ctx, item.idx) {
            continue;
        }
        let ring = visible_item_ring(ctx, item);
        if !preview_has_any(ctx, item.id, item.idx) {
            let blocked = coverage_thumb_key_for_item(ctx, item.id, item.idx)
                .map(|key| preview_retry_blocked_at(ctx, key, retry_now))
                .unwrap_or(false);
            if blocked {
                continue;
            }
            coverage_missing_any.push(RingItem { item, ring });
            ring_missing[ring] = ring_missing[ring].saturating_add(1);
        } else if !preview_has_coverage(ctx, item.id, item.idx) {
            coverage_upgrade.push(RingItem { item, ring });
        }
    }
    let preview_missing_any_count = coverage_missing_any.len() as u32;
    let preview_upgrade_count = coverage_upgrade.len() as u32;
    boost_preview_coverage_budget(ctx, coverage_missing_any.len(), coverage_upgrade.len());
    let coverage_outstanding_target = desired_preview_coverage_outstanding(
        ctx,
        coverage_missing_any.len(),
        coverage_upgrade.len(),
    );

    let has_presence_gaps = !coverage_missing_any.is_empty();
    let ring_lock = update_preview_ring_lock(ctx, &ring_missing, has_presence_gaps);
    let moving_recently = ctx.viewport_runtime().moving_recently;
    let ring_lock_limit = ring_lock.map(|lock| {
        let band = if moving_recently {
            PREVIEW_RING_LOCK_MOVING_BAND
        } else {
            PREVIEW_RING_LOCK_IDLE_BAND
        };
        (lock + band).min(PREVIEW_RING_COUNT - 1)
    });
    let mut pending_quality_dropped = 0u32;
    if has_presence_gaps {
        pending_quality_dropped = drop_pending_quality_previews(ctx);
    }
    let pending_cap_dropped = drop_pending_quality_to_cap(ctx, pending_preview_cap(ctx));
    pending_quality_dropped = pending_quality_dropped.saturating_add(pending_cap_dropped);
    let (mut pending_coverage, _) = pending_preview_class_counts(ctx);

    let pending_pruned_total = pending_pruned.saturating_add(pending_ram_filtered);
    let should_sync_loader =
        moving_recently || pending_pruned_total > 0 || pending_quality_dropped > 0;
    if should_sync_loader {
        let _ = sync_loader_preview_queue_with_pending(ctx);
    }

    let mut coverage_slots = coverage_outstanding_target.saturating_sub(pending_coverage as usize);
    if has_presence_gaps {
        for ring_item in coverage_missing_any.iter().copied() {
            if coverage_slots == 0 {
                break;
            }
            if let Some(limit) = ring_lock_limit {
                if ring_item.ring > limit {
                    continue;
                }
            }
            enqueue_preview_coverage_if_slot(
                ctx,
                ring_item.item,
                &mut coverage_slots,
                &mut pending_coverage,
            );
        }
    } else {
        for ring_item in coverage_upgrade.iter().copied() {
            if coverage_slots == 0 {
                break;
            }
            enqueue_preview_coverage_if_slot(
                ctx,
                ring_item.item,
                &mut coverage_slots,
                &mut pending_coverage,
            );
        }
    }

    let has_coverage_gaps = ctx
        .committed_view
        .visible_items
        .iter()
        .copied()
        .any(|item| {
            if !item_is_ram_media_slot(ctx, &ram_assets, item.idx) {
                return false;
            }
            if !item_has_known_dimensions(ctx, item.idx) {
                return false;
            }
            if preview_has_coverage(ctx, item.id, item.idx) {
                return false;
            }
            coverage_thumb_key_for_item(ctx, item.id, item.idx)
                .map(|key| !preview_retry_blocked_at(ctx, key, retry_now))
                .unwrap_or(true)
        });
    let quality_phase_enabled = !has_coverage_gaps;
    if !has_coverage_gaps {
        let quality_items = build_preview_order_soft_center(ctx, &ctx.committed_view.visible_items);
        for item in quality_items {
            if !item_is_ram_media_slot(ctx, &ram_assets, item.idx) {
                continue;
            }
            render_streaming::handle_preview_quality_request(ctx, item.id, item.idx);
        }
    }
    let (pending_coverage, pending_quality) = pending_preview_class_counts(ctx);
    ctx.quality_stats.record_preview_phase_last(
        preview_missing_any_count,
        preview_upgrade_count,
        has_presence_gaps,
        quality_phase_enabled,
        pending_coverage,
        pending_quality,
        pending_pruned_total,
        pending_quality_dropped,
    );

    let items: Vec<VisibleItem> = ctx.committed_view.visible_items.to_vec();
    let mut no_atlas = 0u32;
    let mut preview_covered = 0u32;
    let mut preview_total = 0u32;
    for item in items {
        if !item_is_ram_media_slot(ctx, &ram_assets, item.idx) {
            continue;
        }
        render_streaming::handle_request(ctx, item.id, item.idx);
        let known_dims = item_has_known_dimensions(ctx, item.idx);
        if known_dims {
            preview_total = preview_total.saturating_add(1);
            if preview_has_coverage(ctx, item.id, item.idx) {
                preview_covered = preview_covered.saturating_add(1);
            }
        }
        if let Some(raw) = ctx.scene.all_items_raw.get(item.idx) {
            if raw.uv_region[2] <= 0.0 {
                no_atlas = no_atlas.saturating_add(1);
            }
        }
    }
    ctx.quality_stats
        .record_visible_preview_coverage_last(preview_covered, preview_total);

    let frame = ctx.frame_count;
    if preview_total == 0 {
        ctx.streaming_runtime.clear_preview_completion_tracking();
    } else if preview_covered < preview_total {
        if ctx.streaming_runtime.preview.not_full_since_frame.is_none() {
            ctx.streaming_runtime.preview.not_full_since_frame = Some(frame);
        }
    } else if let Some(start) = ctx.streaming_runtime.preview.not_full_since_frame.take() {
        let frames = frame.saturating_sub(start);
        ctx.quality_stats.record_preview_full_coverage(frames);
    }
    ctx.quality_stats
        .record_visible_items_last(ctx.committed_view.visible_items.len() as u32, no_atlas);
}
