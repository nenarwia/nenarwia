use std::time::Duration;

use crate::render::atlas::ThumbClass;

use super::StreamingConfig;

const REFERENCE_FRAME_RATE_HZ: f32 = 60.0;
const MAX_COUNT_BURST_REFERENCE_FRAMES: f32 = 4.0;
const MAX_TIME_BURST_REFERENCE_FRAMES: f32 = 4.0;

#[derive(Default)]
pub(crate) struct StreamingBudgetState {
    pub(crate) canvas_media_slot_budget_tokens: f32,
    pub(crate) canvas_media_slot_budget_remaining: usize,
    pub(crate) canvas_media_slot_min_visible_tokens: f32,
    pub(crate) canvas_media_slot_min_visible_remaining: usize,
    pub(crate) thumb_budget_tokens: f32,
    pub(crate) thumb_budget_remaining: usize,
    pub(crate) thumb_coverage_budget_tokens: f32,
    pub(crate) thumb_coverage_budget_remaining: usize,
    pub(crate) upload_budget_tokens: f32,
    pub(crate) upload_budget_remaining: usize,
    pub(crate) canvas_media_slot_cpu_budget_for_update: Option<Duration>,
    pub(crate) upload_cpu_budget_for_update: Option<Duration>,
}

impl StreamingBudgetState {
    pub(super) fn prepare_frame_budgets(
        &mut self,
        streaming: &StreamingConfig,
        camera_moving: bool,
        frame_dt: Duration,
    ) {
        self.canvas_media_slot_budget_remaining = refill_count_budget(
            &mut self.canvas_media_slot_budget_tokens,
            streaming.max_canvas_media_slot_requests_per_frame,
            frame_dt,
        );

        let preview_frame_budget = if camera_moving {
            streaming
                .max_thumb_requests_per_frame
                .min(streaming.max_preview_requests_moving_per_frame)
        } else {
            streaming.max_thumb_requests_per_frame
        };
        self.thumb_budget_remaining = refill_count_budget(
            &mut self.thumb_budget_tokens,
            preview_frame_budget,
            frame_dt,
        );

        let min_coverage = if camera_moving {
            streaming.min_visible_previews_moving_per_frame
        } else {
            streaming.min_visible_previews_per_frame
        };
        self.thumb_coverage_budget_remaining = refill_count_budget(
            &mut self.thumb_coverage_budget_tokens,
            min_coverage,
            frame_dt,
        )
        .min(self.thumb_budget_remaining);
        self.canvas_media_slot_min_visible_remaining = refill_count_budget(
            &mut self.canvas_media_slot_min_visible_tokens,
            streaming.min_visible_canvas_media_slots_per_frame,
            frame_dt,
        )
        .min(self.canvas_media_slot_budget_remaining);
        self.upload_budget_remaining = refill_count_budget(
            &mut self.upload_budget_tokens,
            streaming.max_uploads_per_frame,
            frame_dt,
        );
        self.canvas_media_slot_cpu_budget_for_update =
            scaled_time_budget(streaming.canvas_media_slot_cpu_budget_ms, frame_dt);
        self.upload_cpu_budget_for_update =
            scaled_time_budget(streaming.cpu_budget_ms_upload as f32, frame_dt);
    }

    pub(super) fn consume_thumb_budget(&mut self, class: ThumbClass) -> bool {
        if self.thumb_budget_remaining == 0 {
            return false;
        }

        match class {
            ThumbClass::Coverage => {
                consume_count_budget(
                    &mut self.thumb_budget_tokens,
                    &mut self.thumb_budget_remaining,
                    1,
                );
                if self.thumb_coverage_budget_remaining > 0 {
                    consume_count_budget(
                        &mut self.thumb_coverage_budget_tokens,
                        &mut self.thumb_coverage_budget_remaining,
                        1,
                    );
                }
                true
            }
            ThumbClass::Quality => {
                if self.thumb_budget_remaining <= self.thumb_coverage_budget_remaining {
                    return false;
                }
                consume_count_budget(
                    &mut self.thumb_budget_tokens,
                    &mut self.thumb_budget_remaining,
                    1,
                );
                true
            }
        }
    }

    pub(super) fn consume_canvas_media_slot_budget(&mut self, count: usize) {
        consume_count_budget(
            &mut self.canvas_media_slot_budget_tokens,
            &mut self.canvas_media_slot_budget_remaining,
            count,
        );
    }

    pub(super) fn consume_canvas_media_slot_min_visible_budget(&mut self, count: usize) {
        consume_count_budget(
            &mut self.canvas_media_slot_min_visible_tokens,
            &mut self.canvas_media_slot_min_visible_remaining,
            count,
        );
    }

    pub(super) fn consume_upload_budget(&mut self, count: usize) {
        consume_count_budget(
            &mut self.upload_budget_tokens,
            &mut self.upload_budget_remaining,
            count,
        );
    }
}

fn refill_count_budget(tokens: &mut f32, per_reference_frame: usize, frame_dt: Duration) -> usize {
    if per_reference_frame == 0 {
        *tokens = 0.0;
        return 0;
    }

    let per_reference_frame = per_reference_frame as f32;
    let dt_reference_frames = frame_dt.as_secs_f32().max(0.0) * REFERENCE_FRAME_RATE_HZ;
    let burst_cap = per_reference_frame * MAX_COUNT_BURST_REFERENCE_FRAMES;
    *tokens = (*tokens + per_reference_frame * dt_reference_frames).min(burst_cap);
    tokens.floor() as usize
}

fn consume_count_budget(tokens: &mut f32, remaining: &mut usize, count: usize) {
    if count == 0 {
        return;
    }

    *tokens = (*tokens - count as f32).max(0.0);
    *remaining = remaining.saturating_sub(count);
}

fn scaled_time_budget(per_reference_frame_ms: f32, frame_dt: Duration) -> Option<Duration> {
    if !per_reference_frame_ms.is_finite() || per_reference_frame_ms <= 0.0 {
        return None;
    }

    let dt_reference_frames = frame_dt.as_secs_f32().max(0.0) * REFERENCE_FRAME_RATE_HZ;
    let budget_ms = (per_reference_frame_ms * dt_reference_frames)
        .min(per_reference_frame_ms * MAX_TIME_BURST_REFERENCE_FRAMES);
    Some(Duration::from_secs_f32((budget_ms / 1000.0).max(0.0)))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::render::atlas::ThumbClass;
    use crate::render::context::state::StreamingConfig;

    use super::StreamingBudgetState;

    #[test]
    fn prepare_frame_budgets_respects_motion_specific_limits() {
        let mut budgets = StreamingBudgetState::default();
        let streaming = StreamingConfig {
            max_canvas_media_slot_requests_per_frame: 99,
            max_thumb_requests_per_frame: 40,
            min_visible_previews_per_frame: 12,
            min_visible_previews_moving_per_frame: 7,
            max_preview_requests_moving_per_frame: 15,
            ..StreamingConfig::default()
        };

        budgets.prepare_frame_budgets(&streaming, true, Duration::from_secs_f64(1.0 / 60.0));
        assert_eq!(budgets.canvas_media_slot_budget_remaining, 99);
        assert_eq!(budgets.thumb_budget_remaining, 15);
        assert_eq!(budgets.thumb_coverage_budget_remaining, 7);
        assert_eq!(budgets.canvas_media_slot_min_visible_remaining, 8);
        assert_eq!(
            budgets.upload_budget_remaining,
            streaming.max_uploads_per_frame
        );

        budgets.consume_canvas_media_slot_budget(budgets.canvas_media_slot_budget_remaining);
        budgets.consume_canvas_media_slot_min_visible_budget(
            budgets.canvas_media_slot_min_visible_remaining,
        );
        budgets.consume_upload_budget(budgets.upload_budget_remaining);
        for _ in 0..7 {
            assert!(budgets.consume_thumb_budget(ThumbClass::Coverage));
        }
        for _ in 7..15 {
            assert!(budgets.consume_thumb_budget(ThumbClass::Quality));
        }
        budgets.prepare_frame_budgets(&streaming, false, Duration::from_secs_f64(1.0 / 60.0));
        assert_eq!(budgets.thumb_budget_remaining, 40);
        assert_eq!(budgets.thumb_coverage_budget_remaining, 12);
    }

    #[test]
    fn count_budgets_match_reference_throughput_across_refresh_rates() {
        let streaming = StreamingConfig {
            max_canvas_media_slot_requests_per_frame: 128,
            max_thumb_requests_per_frame: 32,
            min_visible_previews_per_frame: 12,
            min_visible_previews_moving_per_frame: 7,
            max_preview_requests_moving_per_frame: 15,
            max_uploads_per_frame: 200,
            min_visible_canvas_media_slots_per_frame: 24,
            ..StreamingConfig::default()
        };

        let totals_60 = run_budget_totals(&streaming, Duration::from_secs_f64(1.0 / 60.0), 60);
        let totals_144 = run_budget_totals(&streaming, Duration::from_secs_f64(1.0 / 144.0), 144);

        assert_eq!(totals_60.canvas_slots, 128 * 60);
        assert_budget_close(totals_144.canvas_slots, totals_60.canvas_slots, 1);
        assert_budget_close(totals_144.thumbs, totals_60.thumbs, 1);
        assert_budget_close(totals_144.thumb_coverage, totals_60.thumb_coverage, 1);
        assert_budget_close(totals_144.visible_tiles, totals_60.visible_tiles, 1);
        assert_budget_close(totals_144.uploads, totals_60.uploads, 1);
    }

    #[test]
    fn long_stall_burst_is_clamped_to_reference_frames() {
        let mut budgets = StreamingBudgetState::default();
        let streaming = StreamingConfig {
            max_canvas_media_slot_requests_per_frame: 50,
            max_thumb_requests_per_frame: 20,
            min_visible_previews_per_frame: 6,
            min_visible_canvas_media_slots_per_frame: 10,
            max_uploads_per_frame: 30,
            ..StreamingConfig::default()
        };

        budgets.prepare_frame_budgets(&streaming, false, Duration::from_secs(1));

        assert_eq!(budgets.canvas_media_slot_budget_remaining, 200);
        assert_eq!(budgets.thumb_budget_remaining, 80);
        assert_eq!(budgets.thumb_coverage_budget_remaining, 24);
        assert_eq!(budgets.canvas_media_slot_min_visible_remaining, 40);
        assert_eq!(budgets.upload_budget_remaining, 120);
    }

    #[test]
    fn time_budgets_scale_with_dt_and_clamp_after_stall() {
        let mut budgets = StreamingBudgetState::default();
        let streaming = StreamingConfig {
            canvas_media_slot_cpu_budget_ms: 3.0,
            cpu_budget_ms_upload: 2,
            ..StreamingConfig::default()
        };

        budgets.prepare_frame_budgets(&streaming, false, Duration::from_secs_f64(1.0 / 60.0));
        assert_eq!(
            budgets
                .canvas_media_slot_cpu_budget_for_update
                .map(|budget| budget.as_micros()),
            Some(3_000)
        );
        assert_eq!(
            budgets
                .upload_cpu_budget_for_update
                .map(|budget| budget.as_micros()),
            Some(2_000)
        );

        budgets.prepare_frame_budgets(&streaming, false, Duration::from_secs_f64(1.0 / 144.0));
        assert!(matches!(
            budgets
                .canvas_media_slot_cpu_budget_for_update
                .map(|budget| budget.as_micros()),
            Some(1_249..=1_251)
        ));
        assert!(matches!(
            budgets
                .upload_cpu_budget_for_update
                .map(|budget| budget.as_micros()),
            Some(832..=834)
        ));

        budgets.prepare_frame_budgets(&streaming, false, Duration::from_secs(1));
        assert_eq!(
            budgets
                .canvas_media_slot_cpu_budget_for_update
                .map(|budget| budget.as_micros()),
            Some(12_000)
        );
        assert_eq!(
            budgets
                .upload_cpu_budget_for_update
                .map(|budget| budget.as_micros()),
            Some(8_000)
        );
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct BudgetTotals {
        canvas_slots: usize,
        thumbs: usize,
        thumb_coverage: usize,
        visible_tiles: usize,
        uploads: usize,
    }

    fn run_budget_totals(
        streaming: &StreamingConfig,
        frame_dt: Duration,
        frames: usize,
    ) -> BudgetTotals {
        let mut budgets = StreamingBudgetState::default();
        let mut totals = BudgetTotals {
            canvas_slots: 0,
            thumbs: 0,
            thumb_coverage: 0,
            visible_tiles: 0,
            uploads: 0,
        };

        for _ in 0..frames {
            budgets.prepare_frame_budgets(streaming, false, frame_dt);

            let canvas_slots = budgets.canvas_media_slot_budget_remaining;
            let thumbs = budgets.thumb_budget_remaining;
            let thumb_coverage = budgets.thumb_coverage_budget_remaining;
            let visible_tiles = budgets.canvas_media_slot_min_visible_remaining;
            let uploads = budgets.upload_budget_remaining;

            totals.canvas_slots += canvas_slots;
            totals.thumbs += thumbs;
            totals.thumb_coverage += thumb_coverage;
            totals.visible_tiles += visible_tiles;
            totals.uploads += uploads;

            budgets.consume_canvas_media_slot_budget(canvas_slots);
            budgets.consume_canvas_media_slot_min_visible_budget(visible_tiles);
            budgets.consume_upload_budget(uploads);
            for _ in 0..thumb_coverage {
                assert!(budgets.consume_thumb_budget(ThumbClass::Coverage));
            }
            for _ in thumb_coverage..thumbs {
                assert!(budgets.consume_thumb_budget(ThumbClass::Quality));
            }
        }

        totals
    }

    fn assert_budget_close(actual: usize, expected: usize, tolerance: usize) {
        assert!(
            actual.abs_diff(expected) <= tolerance,
            "expected {expected} +/- {tolerance}, got {actual}"
        );
    }
}
