use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::core::loader::mem_cache;
use crate::render::context::state::{RenderContext, VisibleItem};
use crate::render::streaming::canvas_media_slots::calculator::media_world_size_to_pixels;

use super::{ordering::visible_item_ring, PREVIEW_RING_COUNT};

#[derive(Clone, Copy, Debug)]
struct ResidentCandidate {
    asset_key: u64,
    max_px: f32,
    id: u64,
}

pub(super) fn resident_media_slot_assets_for_frame(ctx: &mut RenderContext) -> HashSet<u64> {
    let now = Instant::now();
    if !should_update_slot_residency(ctx, now) {
        return mem_cache::resident_media_slot_asset_keys();
    }

    let grace = if ctx.viewport_runtime().moving_recently {
        ctx.streaming_runtime.residency.grace_moving
    } else {
        ctx.streaming_runtime.residency.grace_idle
    };
    let visible_candidates = collect_visible_resident_candidates(ctx);

    for asset_key in visible_candidates
        .iter()
        .map(|candidate| candidate.asset_key)
    {
        ctx.streaming_runtime
            .residency
            .hot_at
            .insert(asset_key, now);
    }

    prune_expired_hot_assets(&mut ctx.streaming_runtime.residency.hot_at, now, grace);

    let desired_assets = build_desired_resident_asset_list(ctx, &visible_candidates, now);
    let current_assets = mem_cache::resident_media_slot_asset_keys();
    let changed = !same_asset_set(&current_assets, &desired_assets);
    if changed {
        mem_cache::set_ram_media_slot_assets(&desired_assets);
        if ctx.debug_slot_backdrop_enabled {
            ctx.mark_slot_backdrop_dirty();
        }
    }

    ctx.streaming_runtime.residency.last_update_at = Some(now);
    if changed {
        mem_cache::resident_media_slot_asset_keys()
    } else {
        current_assets
    }
}

fn should_update_slot_residency(ctx: &RenderContext, now: Instant) -> bool {
    let Some(last_update_at) = ctx.streaming_runtime.residency.last_update_at else {
        return true;
    };

    if !ctx.viewport_runtime().moving_recently
        && ctx
            .viewport_runtime()
            .last_changed_at
            .is_some_and(|changed_at| changed_at > last_update_at)
    {
        return true;
    }

    now.saturating_duration_since(last_update_at) >= ctx.streaming_runtime.residency.update_interval
}

fn collect_visible_resident_candidates(ctx: &RenderContext) -> Vec<ResidentCandidate> {
    let mut ring_buckets: [Vec<ResidentCandidate>; PREVIEW_RING_COUNT] =
        std::array::from_fn(|_| Vec::new());

    for item in ctx.committed_view.visible_items.iter().copied() {
        let Some(candidate) = resident_candidate_for_item(ctx, item) else {
            continue;
        };
        let ring = visible_item_ring(ctx, item);
        ring_buckets[ring].push(candidate);
    }

    for bucket in ring_buckets.iter_mut() {
        bucket.sort_by(|a, b| {
            b.max_px
                .total_cmp(&a.max_px)
                .then_with(|| a.id.cmp(&b.id))
                .then_with(|| a.asset_key.cmp(&b.asset_key))
        });
    }

    let mut ordered = Vec::with_capacity(ctx.committed_view.visible_items.len());
    let mut seen_assets = HashSet::with_capacity(ctx.committed_view.visible_items.len());
    for bucket in ring_buckets.into_iter() {
        for candidate in bucket {
            if seen_assets.insert(candidate.asset_key) {
                ordered.push(candidate);
            }
        }
    }
    ordered
}

fn resident_candidate_for_item(
    ctx: &RenderContext,
    item: VisibleItem,
) -> Option<ResidentCandidate> {
    let raw = ctx.scene.all_items_raw.get(item.idx)?;
    if raw.data[2] <= 0.0 || raw.data[3] <= 0.0 {
        return None;
    }
    let asset_key = ctx.scene.asset_keys.get(item.idx).copied().unwrap_or(0);
    if asset_key == 0 {
        return None;
    }

    let (obj_px_w, obj_px_h) = media_world_size_to_pixels(ctx, item.idx)?;
    Some(ResidentCandidate {
        asset_key,
        max_px: obj_px_w.max(obj_px_h),
        id: item.id,
    })
}

fn prune_expired_hot_assets(hot_assets: &mut HashMap<u64, Instant>, now: Instant, grace: Duration) {
    hot_assets.retain(|_, last_hot_at| now.saturating_duration_since(*last_hot_at) <= grace);
}

fn build_desired_resident_asset_list(
    ctx: &RenderContext,
    visible_candidates: &[ResidentCandidate],
    now: Instant,
) -> Vec<u64> {
    let mut desired = Vec::with_capacity(
        mem_cache::MAX_RAM_MEDIA_SLOTS
            .min(visible_candidates.len() + ctx.streaming_runtime.residency.hot_at.len()),
    );
    let mut seen = HashSet::with_capacity(desired.capacity());

    for candidate in visible_candidates.iter().copied() {
        if desired.len() >= mem_cache::MAX_RAM_MEDIA_SLOTS {
            return desired;
        }
        if seen.insert(candidate.asset_key) {
            desired.push(candidate.asset_key);
        }
    }

    let mut sticky_assets: Vec<(u64, Instant)> = ctx
        .streaming_runtime
        .residency
        .hot_at
        .iter()
        .map(|(&asset_key, &last_hot_at)| (asset_key, last_hot_at))
        .collect();
    sticky_assets.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    for (asset_key, last_hot_at) in sticky_assets.into_iter() {
        if desired.len() >= mem_cache::MAX_RAM_MEDIA_SLOTS {
            break;
        }
        if last_hot_at > now {
            continue;
        }
        if seen.insert(asset_key) {
            desired.push(asset_key);
        }
    }

    desired
}

fn same_asset_set(current: &HashSet<u64>, desired: &[u64]) -> bool {
    current.len() == desired.len() && desired.iter().all(|asset_key| current.contains(asset_key))
}
