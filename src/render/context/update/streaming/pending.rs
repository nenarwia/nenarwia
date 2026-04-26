use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::render::atlas::ThumbClass;
use crate::render::context::state::RenderContext;
use crate::render::streaming::preview::{thumb_request_id, ThumbRequestKey};

#[inline]
pub(super) fn preview_retry_blocked_at(
    ctx: &RenderContext,
    key: ThumbRequestKey,
    now: Instant,
) -> bool {
    matches!(
        ctx.streaming_runtime.preview.retry_after.get(&key),
        Some(&retry_after) if retry_after > now
    )
}

pub(super) fn prune_preview_retry_after(ctx: &mut RenderContext) {
    let now = Instant::now();
    if ctx.streaming_runtime.preview.retry_after.is_empty() {
        return;
    }
    let prune_interval = RenderContext::duration_for_reference_frames(60);
    let periodic_prune = ctx
        .streaming_runtime
        .preview
        .retry_last_prune_at
        .map(|last| now.saturating_duration_since(last) >= prune_interval)
        .unwrap_or(true);
    if ctx.streaming_runtime.preview.retry_after.len() > 4096 || periodic_prune {
        ctx.streaming_runtime
            .preview
            .retry_after
            .retain(|_, &mut retry_after| retry_after > now);
        ctx.streaming_runtime.preview.retry_last_prune_at = Some(now);
    }
}

pub(super) fn drop_pending_quality_previews(ctx: &mut RenderContext) -> u32 {
    let epoch = ctx.streaming_runtime.stream_epoch;
    let before = ctx.streaming_runtime.preview.pending_slots.len();
    ctx.streaming_runtime
        .preview
        .pending_slots
        .retain(|_, pending| pending.epoch == epoch && pending.class == ThumbClass::Coverage);
    before.saturating_sub(ctx.streaming_runtime.preview.pending_slots.len()) as u32
}

pub(super) fn drop_pending_quality_to_cap(ctx: &mut RenderContext, cap: usize) -> u32 {
    if ctx.streaming_runtime.preview.pending_slots.len() <= cap {
        return 0;
    }
    let mut need_drop = ctx
        .streaming_runtime
        .preview
        .pending_slots
        .len()
        .saturating_sub(cap);
    if need_drop == 0 {
        return 0;
    }

    let epoch = ctx.streaming_runtime.stream_epoch;
    let mut keys = Vec::with_capacity(need_drop);
    for (key, pending) in ctx.streaming_runtime.preview.pending_slots.iter() {
        if pending.epoch == epoch && pending.class == ThumbClass::Quality {
            keys.push(*key);
            need_drop = need_drop.saturating_sub(1);
            if need_drop == 0 {
                break;
            }
        }
    }

    for key in keys.iter().copied() {
        ctx.streaming_runtime.preview.pending_slots.remove(&key);
    }
    keys.len() as u32
}

pub(super) fn prune_pending_previews_to_visible(ctx: &mut RenderContext) -> u32 {
    let epoch = ctx.streaming_runtime.stream_epoch;
    let visible = &ctx.committed_view.visible_ids;
    let before = ctx.streaming_runtime.preview.pending_slots.len();
    let mut keep_tier_per_id = HashMap::<u64, u16>::new();
    for (key, pending) in ctx.streaming_runtime.preview.pending_slots.iter() {
        if pending.epoch != epoch {
            continue;
        }
        let id = thumb_request_id(*key);
        if !visible.contains(&id) {
            continue;
        }
        let entry = keep_tier_per_id.entry(id).or_insert(pending.tier);
        if pending.tier > *entry {
            *entry = pending.tier;
        }
    }

    for (&id, state) in ctx.streaming_runtime.preview.tier_state.iter() {
        if !visible.contains(&id) {
            continue;
        }
        keep_tier_per_id.insert(id, state.target.page_size() as u16);
    }

    ctx.streaming_runtime
        .preview
        .pending_slots
        .retain(|key, pending| {
            if pending.epoch != epoch {
                return false;
            }
            let id = thumb_request_id(*key);
            if !visible.contains(&id) {
                return false;
            }
            keep_tier_per_id
                .get(&id)
                .copied()
                .map(|keep_tier| pending.tier == keep_tier)
                .unwrap_or(false)
        });
    before.saturating_sub(ctx.streaming_runtime.preview.pending_slots.len()) as u32
}

pub(super) fn pending_preview_class_counts(ctx: &RenderContext) -> (u32, u32) {
    let mut coverage = 0u32;
    let mut quality = 0u32;
    let epoch = ctx.streaming_runtime.stream_epoch;
    for pending in ctx
        .streaming_runtime
        .preview
        .pending_slots
        .values()
        .copied()
    {
        if pending.epoch != epoch {
            continue;
        }
        match pending.class {
            ThumbClass::Coverage => coverage = coverage.saturating_add(1),
            ThumbClass::Quality => quality = quality.saturating_add(1),
        }
    }
    (coverage, quality)
}

pub(super) fn boost_preview_coverage_budget(
    ctx: &mut RenderContext,
    missing_any: usize,
    upgrade_needed: usize,
) {
    let moving_recently = ctx.viewport_runtime().moving_recently;
    let dynamic_div = if moving_recently { 48usize } else { 64usize };
    let upgrade_div = if moving_recently { 96usize } else { 128usize };

    let gap_boost = missing_any / dynamic_div;
    let upgrade_boost = if missing_any == 0 {
        upgrade_needed / upgrade_div
    } else {
        0
    };

    let target = ctx
        .streaming_runtime
        .budgets
        .thumb_coverage_budget_remaining
        .saturating_add(gap_boost)
        .saturating_add(upgrade_boost)
        .min(ctx.streaming.max_thumb_requests_per_frame)
        .min(ctx.streaming_runtime.budgets.thumb_budget_remaining);

    ctx.streaming_runtime
        .budgets
        .thumb_coverage_budget_remaining = target;
}

pub(super) fn desired_preview_coverage_outstanding(
    ctx: &RenderContext,
    missing_any: usize,
    upgrade_needed: usize,
) -> usize {
    let moving_recently = ctx.viewport_runtime().moving_recently;
    let effective_preview_budget = if moving_recently {
        ctx.streaming
            .max_thumb_requests_per_frame
            .min(ctx.streaming.max_preview_requests_moving_per_frame)
    } else {
        ctx.streaming.max_thumb_requests_per_frame
    };
    let lead_frames = if moving_recently {
        super::PREVIEW_COVERAGE_LEAD_FRAMES_MOVING
    } else {
        super::PREVIEW_COVERAGE_LEAD_FRAMES_IDLE
    };
    let min_rate = if moving_recently {
        ctx.streaming.min_visible_previews_moving_per_frame
    } else {
        ctx.streaming.min_visible_previews_per_frame
    } as f32;
    let mut target = ((ctx
        .streaming_runtime
        .preview
        .coverage_upload_ema
        .max(min_rate)
        .max(1.0))
        * lead_frames)
        .round() as usize;
    let frame_floor = effective_preview_budget.saturating_mul(if moving_recently {
        super::PREVIEW_COVERAGE_MOVING_QUEUE_FLOOR_FRAMES
    } else {
        6
    });
    target = target.max(frame_floor);
    target = target.saturating_add(missing_any / 32);
    if missing_any == 0 {
        target = target.saturating_add(upgrade_needed / 64);
    }
    let mut clamp_min = ctx.streaming_runtime.preview.coverage_outstanding_min;
    if moving_recently {
        let moving_cap = effective_preview_budget
            .saturating_mul(super::PREVIEW_COVERAGE_MOVING_QUEUE_CAP_FRAMES)
            .max(1);
        clamp_min = clamp_min.min(moving_cap);
        target = target.min(moving_cap);
    }
    target.clamp(
        clamp_min,
        ctx.streaming_runtime.preview.coverage_outstanding_max,
    )
}

pub(super) fn update_preview_ring_lock(
    ctx: &mut RenderContext,
    ring_missing: &[u32; super::PREVIEW_RING_COUNT],
    has_presence_gaps: bool,
) -> Option<usize> {
    if !has_presence_gaps {
        ctx.streaming_runtime.preview.coverage_ring_lock = None;
        return None;
    }
    let first_gap_ring = (0..super::PREVIEW_RING_COUNT).find(|&r| ring_missing[r] > 0)?;
    let mut lock = ctx
        .streaming_runtime
        .preview
        .coverage_ring_lock
        .unwrap_or(first_gap_ring);
    if first_gap_ring < lock {
        lock = first_gap_ring;
    }
    while lock < super::PREVIEW_RING_COUNT && ring_missing[lock] == 0 {
        lock += 1;
    }
    if lock >= super::PREVIEW_RING_COUNT {
        ctx.streaming_runtime.preview.coverage_ring_lock = None;
        return None;
    }
    ctx.streaming_runtime.preview.coverage_ring_lock = Some(lock);
    Some(lock)
}

pub(super) fn sync_loader_preview_queue_with_pending(ctx: &mut RenderContext) -> u32 {
    let mut keep = HashSet::with_capacity(ctx.streaming_runtime.preview.pending_slots.len());
    let epoch = ctx.streaming_runtime.stream_epoch;
    for pending in ctx
        .streaming_runtime
        .preview
        .pending_slots
        .values()
        .copied()
    {
        if pending.epoch == epoch {
            keep.insert((pending.asset_key, pending.tier));
        }
    }
    let (purged_jobs, _canceled_subscribers) =
        ctx.loader.retain_queued_thumbnails_epoch_keys(epoch, &keep);
    purged_jobs as u32
}
