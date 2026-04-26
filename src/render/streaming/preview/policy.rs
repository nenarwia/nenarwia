use crate::render::atlas::{MultiTierAtlas, ThumbTier};
use crate::render::context::state::RenderContext;

pub(crate) const MAX_PENDING_PREVIEW_REQUESTS: usize = 512;
pub(crate) const MIN_PENDING_PREVIEW_REQUESTS: usize = 128;

pub(crate) fn required_thumb_tier(max_px: f32) -> ThumbTier {
    if max_px <= 32.0 {
        ThumbTier::Px32
    } else if max_px <= 64.0 {
        ThumbTier::Px64
    } else if max_px <= 128.0 {
        ThumbTier::Px128
    } else if max_px <= 256.0 {
        ThumbTier::Px256
    } else {
        ThumbTier::Px512
    }
}

pub(crate) fn required_coverage_thumb_tier(max_px: f32) -> ThumbTier {
    if max_px <= 96.0 {
        ThumbTier::Px32
    } else if max_px <= 256.0 {
        ThumbTier::Px64
    } else {
        ThumbTier::Px128
    }
}

pub(crate) fn pick_enabled_tier(atlas: &MultiTierAtlas, required: ThumbTier) -> ThumbTier {
    let tiers = [
        ThumbTier::Px32,
        ThumbTier::Px64,
        ThumbTier::Px128,
        ThumbTier::Px256,
        ThumbTier::Px512,
    ];
    let required_px = required.page_size();

    for tier in tiers.iter() {
        if tier.page_size() >= required_px && atlas.enabled(*tier) {
            return *tier;
        }
    }

    for tier in tiers.iter().rev() {
        if atlas.enabled(*tier) {
            return *tier;
        }
    }

    required
}

pub(crate) fn max_enabled_tier(atlas: &MultiTierAtlas) -> ThumbTier {
    if atlas.enabled(ThumbTier::Px512) {
        ThumbTier::Px512
    } else if atlas.enabled(ThumbTier::Px256) {
        ThumbTier::Px256
    } else if atlas.enabled(ThumbTier::Px128) {
        ThumbTier::Px128
    } else if atlas.enabled(ThumbTier::Px64) {
        ThumbTier::Px64
    } else {
        ThumbTier::Px32
    }
}

pub(crate) fn thumb_ready(ctx: &RenderContext, item_idx: usize, desired: ThumbTier) -> bool {
    let Some(raw) = ctx.scene.all_items_raw.get(item_idx) else {
        return false;
    };
    if raw.uv_region[2] <= 0.0 {
        return false;
    }
    match ThumbTier::decode_uv_x(raw.uv_region[0]) {
        Some(current) => (current as u8) >= (desired as u8),
        None => false,
    }
}

pub(crate) fn pending_preview_cap(ctx: &RenderContext) -> usize {
    let tile_pressure = ctx
        .streaming_runtime
        .canvas_media_slots
        .pending
        .len()
        .saturating_add(ctx.streaming_runtime.canvas_media_slots.queue_visible.len())
        .saturating_add(
            ctx.streaming_runtime
                .canvas_media_slots
                .queue_prefetch
                .len(),
        )
        .saturating_add(ctx.loader.inflight_canvas_media_slots());
    pending_preview_cap_for_tile_pressure(tile_pressure)
}

pub(crate) fn pending_preview_cap_for_tile_pressure(tile_pressure: usize) -> usize {
    if tile_pressure >= 2_048 {
        MIN_PENDING_PREVIEW_REQUESTS
    } else if tile_pressure >= 1_024 {
        (MAX_PENDING_PREVIEW_REQUESTS / 2).max(MIN_PENDING_PREVIEW_REQUESTS)
    } else {
        MAX_PENDING_PREVIEW_REQUESTS
    }
}

#[cfg(test)]
mod tests {
    use super::{
        pending_preview_cap_for_tile_pressure, required_coverage_thumb_tier, required_thumb_tier,
        MAX_PENDING_PREVIEW_REQUESTS, MIN_PENDING_PREVIEW_REQUESTS,
    };
    use crate::render::atlas::ThumbTier;

    #[test]
    fn pending_preview_cap_tracks_tile_pressure() {
        assert_eq!(
            pending_preview_cap_for_tile_pressure(0),
            MAX_PENDING_PREVIEW_REQUESTS
        );
        assert_eq!(
            pending_preview_cap_for_tile_pressure(1_024),
            MAX_PENDING_PREVIEW_REQUESTS / 2
        );
        assert_eq!(
            pending_preview_cap_for_tile_pressure(2_048),
            MIN_PENDING_PREVIEW_REQUESTS
        );
    }

    #[test]
    fn pending_preview_cap_respects_boundaries() {
        assert_eq!(
            pending_preview_cap_for_tile_pressure(1_023),
            MAX_PENDING_PREVIEW_REQUESTS
        );
        assert_eq!(
            pending_preview_cap_for_tile_pressure(2_047),
            MAX_PENDING_PREVIEW_REQUESTS / 2
        );
    }

    #[test]
    fn required_coverage_thumb_tier_thresholds() {
        assert_eq!(required_coverage_thumb_tier(96.0), ThumbTier::Px32);
        assert_eq!(required_coverage_thumb_tier(96.1), ThumbTier::Px64);
        assert_eq!(required_coverage_thumb_tier(256.0), ThumbTier::Px64);
        assert_eq!(required_coverage_thumb_tier(256.1), ThumbTier::Px128);
    }

    #[test]
    fn required_thumb_tier_thresholds() {
        assert_eq!(required_thumb_tier(32.0), ThumbTier::Px32);
        assert_eq!(required_thumb_tier(32.1), ThumbTier::Px64);
        assert_eq!(required_thumb_tier(64.1), ThumbTier::Px128);
        assert_eq!(required_thumb_tier(128.1), ThumbTier::Px256);
        assert_eq!(required_thumb_tier(256.1), ThumbTier::Px512);
    }
}
