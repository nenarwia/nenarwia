use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub(super) struct SpringSample {
    value: f64,
    time: Instant,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Spring {
    current: SpringSample,
    start: SpringSample,
    target: SpringSample,
    exponential: bool,
    stiffness: f64,
    animation_duration: Duration,
}

impl Spring {
    pub(super) fn new(
        initial: f64,
        exponential: bool,
        now: Instant,
        stiffness: f64,
        animation_time_secs: f64,
    ) -> Self {
        let sample = SpringSample {
            value: initial,
            time: now,
        };
        Self {
            current: sample,
            start: sample,
            target: sample,
            exponential,
            stiffness,
            animation_duration: Duration::from_secs_f64(animation_time_secs),
        }
    }

    pub(super) fn current_value(&self) -> f64 {
        self.current.value
    }

    pub(super) fn target_value(&self) -> f64 {
        self.target.value
    }

    #[cfg(test)]
    pub(super) fn stiffness(&self) -> f64 {
        self.stiffness
    }

    #[cfg(test)]
    pub(super) fn animation_duration(&self) -> Duration {
        self.animation_duration
    }

    pub(super) fn reset_to(&mut self, value: f64, now: Instant) {
        let sample = SpringSample { value, time: now };
        self.current = sample;
        self.start = sample;
        self.target = sample;
    }

    pub(super) fn spring_to(&mut self, value: f64, now: Instant) {
        self.update(now);
        self.start = self.current;
        self.target = SpringSample {
            value,
            time: now + self.animation_duration,
        };
    }

    pub(super) fn shift_by(&mut self, delta: f64) {
        self.start.value += delta;
        self.target.value += delta;
    }

    pub(super) fn update(&mut self, now: Instant) -> bool {
        self.current.time = now;

        if now >= self.target.time || self.target.time <= self.start.time {
            self.current.value = self.target.value;
            return false;
        }

        let total = self
            .target
            .time
            .saturating_duration_since(self.start.time)
            .as_secs_f64();
        if total <= 0.0 {
            self.current.value = self.target.value;
            return false;
        }

        let progress = now.saturating_duration_since(self.start.time).as_secs_f64() / total;
        let eased = spring_transform(self.stiffness, progress.clamp(0.0, 1.0));

        if self.exponential {
            let start = self.start.value.max(f64::MIN_POSITIVE).ln();
            let target = self.target.value.max(f64::MIN_POSITIVE).ln();
            self.current.value = (start + (target - start) * eased).exp();
        } else {
            self.current.value = self.start.value + (self.target.value - self.start.value) * eased;
        }

        self.current.value != self.target.value
    }

    pub(super) fn is_at_target(&self) -> bool {
        self.current.value == self.target.value && self.current.time >= self.target.time
    }
}

pub(super) fn spring_transform(stiffness: f64, progress: f64) -> f64 {
    (1.0 - (-stiffness * progress).exp()) / (1.0 - (-stiffness).exp())
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::{spring_transform, Spring};
    use crate::spatial::navigation::{SPRING_ANIMATION_TIME_SECS, SPRING_STIFFNESS};

    #[test]
    fn midpoint_matches_expected_curve_for_exponential_spring() {
        let start = Instant::now();
        let mut spring = Spring::new(
            1.0,
            true,
            start,
            SPRING_STIFFNESS,
            SPRING_ANIMATION_TIME_SECS,
        );
        spring.spring_to(4.0, start);

        let sample_time = start + spring.animation_duration() / 2;
        spring.update(sample_time);

        let expected = (1.0f64.ln()
            + (4.0f64.ln() - 1.0f64.ln()) * spring_transform(SPRING_STIFFNESS, 0.5))
        .exp();
        assert!((spring.current_value() - expected).abs() < 1.0e-12);
    }

    #[test]
    fn update_snaps_to_target_after_duration() {
        let start = Instant::now();
        let mut spring = Spring::new(
            0.0,
            false,
            start,
            SPRING_STIFFNESS,
            SPRING_ANIMATION_TIME_SECS,
        );
        spring.spring_to(10.0, start);

        spring.update(start + spring.animation_duration());

        assert_eq!(spring.current_value(), 10.0);
        assert!(spring.is_at_target());
    }
}
