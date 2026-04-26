use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotInteractionTransition {
    None,
    TurnedOff,
    TurnedOn,
}

#[derive(Clone, Copy, Debug)]
pub struct SlotInteractionGate {
    pub enabled: bool,
    pub off_visible_threshold: usize,
    pub on_immediate_visible_threshold: usize,
    pub on_delay_frames: u64,
    pub on_delay: Duration,
    pub pending_on_since: Option<Instant>,
}

impl SlotInteractionGate {
    pub fn new(
        off_visible_threshold: usize,
        on_immediate_visible_threshold: usize,
        on_delay_frames: u64,
    ) -> Self {
        let off_visible_threshold = off_visible_threshold.max(2);
        let on_immediate_visible_threshold = on_immediate_visible_threshold
            .min(off_visible_threshold.saturating_sub(1))
            .max(1);
        Self {
            enabled: true,
            off_visible_threshold,
            on_immediate_visible_threshold,
            on_delay_frames,
            on_delay: reference_frames_to_duration(on_delay_frames),
            pending_on_since: None,
        }
    }

    pub fn update(&mut self, now: Instant, visible_count: usize) -> SlotInteractionTransition {
        if self.enabled {
            self.pending_on_since = None;
            if visible_count > self.off_visible_threshold {
                self.enabled = false;
                return SlotInteractionTransition::TurnedOff;
            }
            return SlotInteractionTransition::None;
        }

        if visible_count < self.on_immediate_visible_threshold {
            self.enabled = true;
            self.pending_on_since = None;
            return SlotInteractionTransition::TurnedOn;
        }

        if visible_count <= self.off_visible_threshold {
            let since = self.pending_on_since.get_or_insert(now);
            if now.saturating_duration_since(*since) >= self.on_delay {
                self.enabled = true;
                self.pending_on_since = None;
                return SlotInteractionTransition::TurnedOn;
            }
            return SlotInteractionTransition::None;
        }

        self.pending_on_since = None;
        SlotInteractionTransition::None
    }

    pub fn pending_on_elapsed_frames(&self, now: Instant) -> Option<u64> {
        self.pending_on_since
            .map(|since| duration_to_reference_frames(now.saturating_duration_since(since)))
    }
}

fn reference_frames_to_duration(frames: u64) -> Duration {
    Duration::from_secs_f64(frames as f64 / 60.0)
}

fn duration_to_reference_frames(duration: Duration) -> u64 {
    (duration.as_secs_f64() * 60.0).floor() as u64
}

#[cfg(test)]
mod slot_interaction_gate_tests {
    use std::time::{Duration, Instant};

    use super::{SlotInteractionGate, SlotInteractionTransition};

    #[test]
    fn turns_off_immediately_above_off_threshold() {
        let mut gate = SlotInteractionGate::new(25_000, 24_000, 24);
        let transition = gate.update(Instant::now(), 25_001);
        assert_eq!(transition, SlotInteractionTransition::TurnedOff);
        assert!(!gate.enabled);
    }

    #[test]
    fn turns_on_immediately_below_on_immediate_threshold() {
        let mut gate = SlotInteractionGate::new(25_000, 24_000, 24);
        let start = Instant::now();
        let _ = gate.update(start, 30_000);
        assert!(!gate.enabled);
        let transition = gate.update(start + Duration::from_millis(1), 23_999);
        assert_eq!(transition, SlotInteractionTransition::TurnedOn);
        assert!(gate.enabled);
    }

    #[test]
    fn turns_on_with_delay_inside_transition_band() {
        let mut gate = SlotInteractionGate::new(25_000, 24_000, 3);
        let start = Instant::now();
        let _ = gate.update(start, 25_500);
        assert!(!gate.enabled);

        assert_eq!(
            gate.update(start + Duration::from_millis(1), 25_000),
            SlotInteractionTransition::None
        );
        assert_eq!(
            gate.update(start + Duration::from_millis(20), 24_500),
            SlotInteractionTransition::None
        );
        assert_eq!(
            gate.update(start + Duration::from_millis(35), 24_200),
            SlotInteractionTransition::None
        );
        assert_eq!(
            gate.update(start + Duration::from_millis(51), 24_100),
            SlotInteractionTransition::TurnedOn
        );
        assert!(gate.enabled);
    }

    #[test]
    fn clears_pending_on_timer_when_overloaded_again() {
        let mut gate = SlotInteractionGate::new(25_000, 24_000, 4);
        let start = Instant::now();
        let _ = gate.update(start, 26_000);
        assert!(!gate.enabled);

        assert_eq!(
            gate.update(start + Duration::from_millis(1), 25_000),
            SlotInteractionTransition::None
        );
        assert_eq!(
            gate.pending_on_since,
            Some(start + Duration::from_millis(1))
        );
        assert_eq!(
            gate.update(start + Duration::from_millis(10), 26_000),
            SlotInteractionTransition::None
        );
        assert_eq!(gate.pending_on_since, None);
    }

    #[test]
    fn pending_elapsed_frames_follow_reference_frame_time() {
        let mut gate = SlotInteractionGate::new(25_000, 24_000, 24);
        let start = Instant::now();
        let _ = gate.update(start, 26_000);
        let _ = gate.update(start + Duration::from_millis(1), 25_000);

        assert_eq!(
            gate.pending_on_elapsed_frames(start + Duration::from_millis(50)),
            Some(2)
        );
    }
}
