use crate::render::atlas::ThumbClass;
#[derive(Clone, Copy, Debug, Default)]
pub struct QualityStats {
    pub lod_clamps: u32,
    pub tier_downgrades: u32,
    pub max_lod_undersample: f32,
    pub max_tier_undersample: f32,
    pub visible_undersample_sum: f32,
    pub visible_undersample_max: f32,
    pub visible_undersample_samples: u32,
    pub visible_missing_tiles: u32,
    pub visible_total_tiles: u32,
    pub max_visible_missing_ratio: f32,
    pub ttfq_samples: u32,
    pub ttfq_frames_sum: u64,
    pub ttfq_frames_max: u64,

    // Per-frame diagnostics (last frame only).
    pub visible_tiles_last: u32,
    pub visible_missing_last: u32,
    pub visible_items_last: u32,
    pub visible_no_atlas_last: u32,
    pub tile_evictions_last: u32,
    pub page_dir_evictions_last: u32,
    pub preview_evictions_last: u32,
    pub visible_preview_covered_last: u32,
    pub visible_preview_total_last: u32,
    pub visible_preview_coverage_ratio_last: f32,
    pub preview_missing_any_last: u32,
    pub preview_upgrade_needed_last: u32,
    pub preview_presence_gaps_last: u32,
    pub preview_quality_phase_enabled_last: u32,
    pub preview_pending_coverage_last: u32,
    pub preview_pending_quality_last: u32,
    pub preview_pending_pruned_last: u32,
    pub preview_pending_quality_dropped_last: u32,
    pub preview_upload_applied_coverage_last: u32,
    pub preview_upload_applied_quality_last: u32,
    pub preview_upload_dropped_epoch_last: u32,
    pub preview_upload_dropped_not_pending_last: u32,
    pub preview_upload_dropped_missing_last: u32,
    pub preview_upload_dropped_no_slot_last: u32,
    pub preview_full_coverage_frames_last: u64,
    pub preview_full_coverage_samples: u32,
    pub preview_full_coverage_frames_sum: u64,
    pub preview_full_coverage_frames_max: u64,
    pub feedback_pages_last: u32,
    pub feedback_overflow_last: u32,
    pub feedback_latency_last: u32,
}

impl QualityStats {
    pub fn reset_frame_counters(&mut self) {
        self.visible_tiles_last = 0;
        self.visible_missing_last = 0;
        self.visible_items_last = 0;
        self.visible_no_atlas_last = 0;
        self.tile_evictions_last = 0;
        self.page_dir_evictions_last = 0;
        self.preview_evictions_last = 0;
        self.visible_preview_covered_last = 0;
        self.visible_preview_total_last = 0;
        self.visible_preview_coverage_ratio_last = 0.0;
        self.preview_missing_any_last = 0;
        self.preview_upgrade_needed_last = 0;
        self.preview_presence_gaps_last = 0;
        self.preview_quality_phase_enabled_last = 0;
        self.preview_pending_coverage_last = 0;
        self.preview_pending_quality_last = 0;
        self.preview_pending_pruned_last = 0;
        self.preview_pending_quality_dropped_last = 0;
        self.preview_upload_applied_coverage_last = 0;
        self.preview_upload_applied_quality_last = 0;
        self.preview_upload_dropped_epoch_last = 0;
        self.preview_upload_dropped_not_pending_last = 0;
        self.preview_upload_dropped_missing_last = 0;
        self.preview_upload_dropped_no_slot_last = 0;
        self.preview_full_coverage_frames_last = 0;
        self.feedback_pages_last = 0;
        self.feedback_overflow_last = 0;
        self.feedback_latency_last = 0;
    }

    pub fn record_lod_clamp(&mut self, undersample: f32) {
        self.lod_clamps = self.lod_clamps.saturating_add(1);
        if undersample > self.max_lod_undersample {
            self.max_lod_undersample = undersample;
        }
    }

    pub fn record_tier_downgrade(&mut self, undersample: f32) {
        self.tier_downgrades = self.tier_downgrades.saturating_add(1);
        if undersample > self.max_tier_undersample {
            self.max_tier_undersample = undersample;
        }
    }

    pub fn record_visible_undersample(&mut self, ratio: f32) {
        if !ratio.is_finite() || ratio <= 0.0 {
            return;
        }
        self.visible_undersample_samples = self.visible_undersample_samples.saturating_add(1);
        self.visible_undersample_sum += ratio;
        if ratio > self.visible_undersample_max {
            self.visible_undersample_max = ratio;
        }
    }

    pub fn record_visible_tiles(&mut self, missing: u32, total: u32) {
        if total == 0 {
            return;
        }
        self.visible_missing_tiles = self.visible_missing_tiles.saturating_add(missing);
        self.visible_total_tiles = self.visible_total_tiles.saturating_add(total);
        let ratio = missing as f32 / total as f32;
        if ratio > self.max_visible_missing_ratio {
            self.max_visible_missing_ratio = ratio;
        }
    }

    pub fn record_ttfq(&mut self, frames: u64) {
        self.ttfq_samples = self.ttfq_samples.saturating_add(1);
        self.ttfq_frames_sum = self.ttfq_frames_sum.saturating_add(frames);
        if frames > self.ttfq_frames_max {
            self.ttfq_frames_max = frames;
        }
    }

    pub fn record_visible_tiles_last(&mut self, missing: u32, total: u32) {
        self.visible_missing_last = self.visible_missing_last.saturating_add(missing);
        self.visible_tiles_last = self.visible_tiles_last.saturating_add(total);
    }

    pub fn record_visible_items_last(&mut self, visible: u32, no_atlas: u32) {
        self.visible_items_last = visible;
        self.visible_no_atlas_last = no_atlas;
    }

    pub fn record_tile_eviction(&mut self) {
        self.tile_evictions_last = self.tile_evictions_last.saturating_add(1);
    }

    pub fn record_page_dir_evictions(&mut self, count: u32) {
        self.page_dir_evictions_last = self.page_dir_evictions_last.saturating_add(count);
    }

    pub fn record_preview_eviction(&mut self) {
        self.preview_evictions_last = self.preview_evictions_last.saturating_add(1);
    }

    pub fn record_visible_preview_coverage_last(&mut self, covered: u32, total: u32) {
        self.visible_preview_covered_last = covered;
        self.visible_preview_total_last = total;
        self.visible_preview_coverage_ratio_last = if total == 0 {
            1.0
        } else {
            covered as f32 / total as f32
        };
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_preview_phase_last(
        &mut self,
        missing_any: u32,
        upgrade_needed: u32,
        presence_gaps: bool,
        quality_phase_enabled: bool,
        pending_coverage: u32,
        pending_quality: u32,
        pending_pruned: u32,
        pending_quality_dropped: u32,
    ) {
        self.preview_missing_any_last = missing_any;
        self.preview_upgrade_needed_last = upgrade_needed;
        self.preview_presence_gaps_last = if presence_gaps { 1 } else { 0 };
        self.preview_quality_phase_enabled_last = if quality_phase_enabled { 1 } else { 0 };
        self.preview_pending_coverage_last = pending_coverage;
        self.preview_pending_quality_last = pending_quality;
        self.preview_pending_pruned_last = pending_pruned;
        self.preview_pending_quality_dropped_last = pending_quality_dropped;
    }

    pub fn record_preview_upload_applied(&mut self, class: ThumbClass) {
        match class {
            ThumbClass::Coverage => {
                self.preview_upload_applied_coverage_last =
                    self.preview_upload_applied_coverage_last.saturating_add(1);
            }
            ThumbClass::Quality => {
                self.preview_upload_applied_quality_last =
                    self.preview_upload_applied_quality_last.saturating_add(1);
            }
        }
    }

    pub fn record_preview_upload_drop_epoch(&mut self) {
        self.preview_upload_dropped_epoch_last =
            self.preview_upload_dropped_epoch_last.saturating_add(1);
    }

    pub fn record_preview_upload_drop_not_pending(&mut self) {
        self.preview_upload_dropped_not_pending_last = self
            .preview_upload_dropped_not_pending_last
            .saturating_add(1);
    }

    pub fn record_preview_upload_drop_missing(&mut self) {
        self.preview_upload_dropped_missing_last =
            self.preview_upload_dropped_missing_last.saturating_add(1);
    }

    pub fn record_preview_upload_drop_no_slot(&mut self) {
        self.preview_upload_dropped_no_slot_last =
            self.preview_upload_dropped_no_slot_last.saturating_add(1);
    }

    pub fn record_preview_full_coverage(&mut self, frames: u64) {
        self.preview_full_coverage_frames_last = frames;
        self.preview_full_coverage_samples = self.preview_full_coverage_samples.saturating_add(1);
        self.preview_full_coverage_frames_sum =
            self.preview_full_coverage_frames_sum.saturating_add(frames);
        if frames > self.preview_full_coverage_frames_max {
            self.preview_full_coverage_frames_max = frames;
        }
    }
}
