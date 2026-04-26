use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

const REFERENCE_FRAME_RATE_HZ: f64 = 60.0;

pub(crate) struct SlotResidencyRuntimeState {
    pub(crate) hot_at: HashMap<u64, Instant>,
    pub(crate) last_update_at: Option<Instant>,
    // Kept as 60 Hz reference-frame values for config compatibility and diagnostics.
    #[allow(dead_code)]
    pub(crate) update_interval_frames: u64,
    pub(crate) update_interval: Duration,
    #[allow(dead_code)]
    pub(crate) grace_frames_idle: u64,
    pub(crate) grace_idle: Duration,
    #[allow(dead_code)]
    pub(crate) grace_frames_moving: u64,
    pub(crate) grace_moving: Duration,
}

impl SlotResidencyRuntimeState {
    pub(super) fn new(
        update_interval_frames: u64,
        grace_frames_idle: u64,
        grace_frames_moving: u64,
    ) -> Self {
        Self {
            hot_at: HashMap::new(),
            last_update_at: None,
            update_interval_frames,
            update_interval: duration_for_reference_frames(update_interval_frames),
            grace_frames_idle,
            grace_idle: duration_for_reference_frames(grace_frames_idle),
            grace_frames_moving,
            grace_moving: duration_for_reference_frames(grace_frames_moving),
        }
    }

    pub(super) fn reset(&mut self) {
        self.hot_at.clear();
        self.last_update_at = None;
    }

    pub(super) fn remove_deleted_assets(&mut self, asset_keys: &HashSet<u64>) {
        self.hot_at
            .retain(|asset_key, _at| !asset_keys.contains(asset_key));
    }
}

fn duration_for_reference_frames(frames: u64) -> Duration {
    Duration::from_secs_f64(frames as f64 / REFERENCE_FRAME_RATE_HZ)
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::SlotResidencyRuntimeState;

    #[test]
    fn new_preserves_reference_frame_diagnostics_and_durations() {
        let state = SlotResidencyRuntimeState::new(12, 60, 24);

        assert_eq!(state.update_interval_frames, 12);
        assert_eq!(state.update_interval.as_millis(), 200);
        assert_eq!(state.grace_frames_idle, 60);
        assert_eq!(state.grace_idle.as_secs(), 1);
        assert_eq!(state.grace_frames_moving, 24);
        assert_eq!(state.grace_moving.as_millis(), 400);
    }

    #[test]
    fn reset_and_remove_deleted_assets_only_touch_hot_asset_tracking() {
        let now = Instant::now();
        let mut state = SlotResidencyRuntimeState::new(12, 60, 24);
        state.hot_at.insert(77, now);
        state.last_update_at = Some(now);

        let asset_keys = [77u64].into_iter().collect();
        state.remove_deleted_assets(&asset_keys);
        assert!(state.hot_at.is_empty());
        assert_eq!(state.last_update_at, Some(now));

        state.hot_at.insert(88, now);
        state.reset();
        assert!(state.hot_at.is_empty());
        assert_eq!(state.last_update_at, None);
    }
}
