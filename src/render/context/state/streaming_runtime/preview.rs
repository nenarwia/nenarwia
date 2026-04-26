use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::render::streaming::preview::ThumbRequestKey;

use crate::render::context::state::{PendingThumbRequest, PreviewTierState};

pub(crate) struct PreviewRuntimeState {
    pub(crate) not_full_since_frame: Option<u64>,
    pub(crate) pending_slots: HashMap<ThumbRequestKey, PendingThumbRequest>,
    pub(crate) tier_state: HashMap<u64, PreviewTierState>,
    pub(crate) retry_after: HashMap<ThumbRequestKey, Instant>,
    pub(crate) retry_last_prune_at: Option<Instant>,
    pub(crate) coverage_upload_ema: f32,
    pub(crate) coverage_outstanding_min: usize,
    pub(crate) coverage_outstanding_max: usize,
    pub(crate) coverage_ring_lock: Option<usize>,
}

impl PreviewRuntimeState {
    pub(super) fn new(
        coverage_upload_ema: f32,
        coverage_outstanding_min: usize,
        coverage_outstanding_max: usize,
    ) -> Self {
        Self {
            not_full_since_frame: None,
            pending_slots: HashMap::new(),
            tier_state: HashMap::new(),
            retry_after: HashMap::new(),
            retry_last_prune_at: None,
            coverage_upload_ema,
            coverage_outstanding_min,
            coverage_outstanding_max,
            coverage_ring_lock: None,
        }
    }

    pub(super) fn clear_pending_work(&mut self) {
        self.pending_slots.clear();
    }

    pub(super) fn clear_completion_tracking(&mut self) {
        self.not_full_since_frame = None;
    }

    pub(super) fn clear_planning_state(&mut self) {
        self.not_full_since_frame = None;
        self.coverage_ring_lock = None;
    }

    pub(super) fn clear_cache_state(&mut self) {
        self.tier_state.clear();
        self.retry_after.clear();
        self.retry_last_prune_at = None;
    }

    pub(super) fn remove_deleted_assets(
        &mut self,
        asset_keys: &HashSet<u64>,
        slot_ids: &HashSet<u64>,
    ) {
        self.pending_slots.retain(|key, pending| {
            !slot_ids.contains(&key.id) && !asset_keys.contains(&pending.asset_key)
        });
        self.tier_state.retain(|id, _state| !slot_ids.contains(id));
        self.retry_after
            .retain(|key, _frame| !slot_ids.contains(&key.id));
    }

    pub(super) fn has_pending_current(&self, epoch: u64) -> bool {
        self.pending_slots
            .values()
            .any(|pending| pending.epoch == epoch)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::{Duration, Instant};

    use crate::render::atlas::{ThumbClass, ThumbTier};
    use crate::render::streaming::preview::thumb_request_key;

    use super::PreviewRuntimeState;
    use crate::render::context::state::{PendingThumbRequest, PreviewTierState};

    fn make_preview_state() -> PreviewRuntimeState {
        let now = Instant::now();
        let mut pending_slots = HashMap::new();
        pending_slots.insert(
            thumb_request_key(11, ThumbTier::Px128),
            PendingThumbRequest {
                epoch: 9,
                class: ThumbClass::Coverage,
                asset_key: 77,
                tier: 128,
            },
        );

        let mut tier_state = HashMap::new();
        tier_state.insert(
            11,
            PreviewTierState {
                target: ThumbTier::Px128,
                display: Some(ThumbTier::Px64),
                pending: Some(ThumbTier::Px128),
            },
        );

        let mut retry_after = HashMap::new();
        retry_after.insert(
            thumb_request_key(11, ThumbTier::Px128),
            now + Duration::from_millis(550),
        );

        PreviewRuntimeState {
            not_full_since_frame: Some(44),
            pending_slots,
            tier_state,
            retry_after,
            retry_last_prune_at: None,
            coverage_upload_ema: 0.0,
            coverage_outstanding_min: 4,
            coverage_outstanding_max: 64,
            coverage_ring_lock: Some(2),
        }
    }

    #[test]
    fn clear_preview_planning_state_resets_runtime_markers_only() {
        let mut preview = make_preview_state();

        preview.clear_planning_state();

        assert_eq!(preview.not_full_since_frame, None);
        assert_eq!(preview.coverage_ring_lock, None);
        assert_eq!(preview.pending_slots.len(), 1);
        assert_eq!(preview.tier_state.len(), 1);
    }

    #[test]
    fn remove_deleted_assets_prunes_pending_tiers_and_retry_state() {
        let mut preview = make_preview_state();
        let slot_ids = [11u64].into_iter().collect();
        let asset_keys = [77u64].into_iter().collect();

        preview.remove_deleted_assets(&asset_keys, &slot_ids);

        assert!(preview.pending_slots.is_empty());
        assert!(preview.tier_state.is_empty());
        assert!(preview.retry_after.is_empty());
    }
}
