use std::time::{Duration, Instant};

use crate::render::atlas::{ThumbClass, ThumbTier};
use crate::render::cache::CanvasMediaSlotId;
use crate::render::context::state::{PendingThumbRequest, PreviewTierState, SlotInteractionGate};
use crate::render::streaming::canvas_media_slots::CanvasMediaSlotQueueItem;
use crate::render::streaming::preview::thumb_request_key;

use super::{StreamingRuntimeInit, StreamingRuntimeState};

fn sample_tile_id() -> CanvasMediaSlotId {
    CanvasMediaSlotId {
        asset_key: 77,
        lod: 2,
        x: 3,
        y: 4,
    }
}

fn sample_queue_item() -> CanvasMediaSlotQueueItem {
    CanvasMediaSlotQueueItem {
        id: 11,
        asset_key: 77,
        item_idx: 5,
        lod: 2,
        x: 3,
        y: 4,
        prio: 10,
        is_prefetch: false,
        epoch: 9,
        queued_frame: 12,
    }
}

fn make_runtime() -> StreamingRuntimeState {
    let now = Instant::now();
    let mut runtime = StreamingRuntimeState::new(StreamingRuntimeInit {
        last_epoch_zoom: 1.0,
        preview_coverage_upload_ema: 0.0,
        preview_coverage_outstanding_min: 4,
        preview_coverage_outstanding_max: 64,
        slot_residency_update_interval_frames: 12,
        slot_residency_grace_frames_idle: 60,
        slot_residency_grace_frames_moving: 24,
        slot_interaction_gate: SlotInteractionGate::new(25_000, 24_000, 24),
    });
    runtime.stream_epoch = 9;
    runtime.preview.not_full_since_frame = Some(44);
    runtime.preview.pending_slots.insert(
        thumb_request_key(11, ThumbTier::Px128),
        PendingThumbRequest {
            epoch: 9,
            class: ThumbClass::Coverage,
            asset_key: 77,
            tier: 128,
        },
    );
    runtime.preview.tier_state.insert(
        11,
        PreviewTierState {
            target: ThumbTier::Px128,
            display: Some(ThumbTier::Px64),
            pending: Some(ThumbTier::Px128),
        },
    );
    runtime.preview.retry_after.insert(
        thumb_request_key(11, ThumbTier::Px128),
        now + Duration::from_millis(550),
    );
    runtime
        .canvas_media_slots
        .pending
        .insert(sample_tile_id(), 9);
    runtime
        .canvas_media_slots
        .queue_visible
        .push(sample_queue_item());
    let mut prefetch_item = sample_queue_item();
    prefetch_item.is_prefetch = true;
    runtime
        .canvas_media_slots
        .queue_prefetch
        .insert(prefetch_item);
    runtime.residency.hot_at.insert(77, now);
    runtime.residency.last_update_at = Some(now);
    runtime.preview.coverage_ring_lock = Some(2);
    runtime
}

#[test]
fn clear_all_pending_work_leaves_preview_metadata_intact() {
    let mut state = make_runtime();

    state.clear_all_pending_work();

    assert!(state.preview.pending_slots.is_empty());
    assert!(state.canvas_media_slots.pending.is_empty());
    assert!(state.canvas_media_slots.queue_visible.is_empty());
    assert!(state.canvas_media_slots.queue_prefetch.is_empty());
    assert_eq!(state.preview.not_full_since_frame, Some(44));
    assert_eq!(state.preview.coverage_ring_lock, Some(2));
    assert_eq!(state.preview.tier_state.len(), 1);
    assert_eq!(state.preview.retry_after.len(), 1);
}

#[test]
fn remove_deleted_assets_prunes_preview_tile_and_residency_state() {
    let mut state = make_runtime();
    let slot_ids = [11u64].into_iter().collect();
    let asset_keys = [77u64].into_iter().collect();

    state.remove_deleted_assets(&asset_keys, &slot_ids);

    assert!(state.preview.pending_slots.is_empty());
    assert!(state.preview.tier_state.is_empty());
    assert!(state.preview.retry_after.is_empty());
    assert!(state.canvas_media_slots.pending.is_empty());
    assert!(state.canvas_media_slots.queue_visible.is_empty());
    assert!(state.canvas_media_slots.queue_prefetch.is_empty());
    assert!(state.residency.hot_at.is_empty());
}

#[test]
fn current_epoch_helpers_delegate_to_grouped_state() {
    let mut state = make_runtime();
    assert!(state.has_pending_slots_current());
    assert!(state.has_pending_canvas_media_slots_current());

    state.stream_epoch = 10;
    assert!(!state.has_pending_slots_current());
    assert!(!state.has_pending_canvas_media_slots_current());
}

#[test]
fn constructor_seeds_grouped_runtime_state() {
    let state = StreamingRuntimeState::new(StreamingRuntimeInit {
        last_epoch_zoom: 2.5,
        preview_coverage_upload_ema: 3.0,
        preview_coverage_outstanding_min: 6,
        preview_coverage_outstanding_max: 18,
        slot_residency_update_interval_frames: 6,
        slot_residency_grace_frames_idle: 24,
        slot_residency_grace_frames_moving: 12,
        slot_interaction_gate: SlotInteractionGate::new(25_000, 24_000, 24),
    });

    assert_eq!(state.stream_epoch, 0);
    assert_eq!(state.last_epoch_zoom, 2.5);
    assert_eq!(state.preview.coverage_upload_ema, 3.0);
    assert_eq!(state.preview.coverage_outstanding_min, 6);
    assert_eq!(state.preview.coverage_outstanding_max, 18);
    assert_eq!(state.residency.update_interval_frames, 6);
    assert_eq!(state.residency.grace_frames_idle, 24);
    assert_eq!(state.residency.grace_frames_moving, 12);
}
