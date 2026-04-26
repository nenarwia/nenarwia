use std::time::Instant;

use crate::render::context::state::RenderContext;
use std::collections::HashSet;

fn stage0_log_enabled() -> bool {
    use std::sync::OnceLock;
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        let val = std::env::var("CANVAS_STAGE0_LOG")
            .unwrap_or_default()
            .to_lowercase();
        matches!(val.as_str(), "1" | "true" | "yes" | "on")
    })
}

impl RenderContext {
    fn center_pan_delta_since_soft_reset_px(&self) -> f32 {
        self.viewport_runtime()
            .center_pan_delta_since_soft_reset_px(self.view_metrics(), self.view())
    }

    pub(crate) fn bump_stream_epoch_hard(&mut self) {
        let epoch = self.streaming_runtime.advance_epoch();
        self.loader.set_epoch(epoch);
        self.streaming_runtime.clear_all_pending_work();
        self.clear_quality_visibility_tracking();
        self.streaming_runtime.clear_preview_planning_state();
        let view = self.view();
        self.viewport_runtime_mut().last_soft_reset_center = (view.center.x, view.center.y);
    }

    fn bump_stream_epoch_soft(&mut self) {
        let epoch = self.streaming_runtime.advance_epoch();
        self.loader.set_epoch(epoch);

        // After an epoch bump, old requests are canceled by workers.
        // Keeping old pending/queues causes stale entries to accumulate.
        self.streaming_runtime.clear_all_pending_work();
        self.streaming_runtime.clear_preview_planning_state();
        let view = self.view();
        self.viewport_runtime_mut().last_soft_reset_center = (view.center.x, view.center.y);
    }

    fn bump_preview_soft_reset(&mut self, pan_px: f32, cur_bucket: i32, last_bucket: i32) {
        let pending_thumbs = self.streaming_runtime.preview.pending_slots.len() as u32;
        if pending_thumbs > 0 {
            self.streaming_runtime.clear_thumbnail_work();
        }
        self.streaming_runtime.clear_preview_planning_state();
        let view = self.view();
        let frame_count = self.frame_count;
        let now = Instant::now();
        {
            let runtime = self.viewport_runtime_mut();
            runtime.last_soft_reset_center = (view.center.x, view.center.y);
            runtime.last_preview_soft_reset_frame = frame_count;
            runtime.last_preview_soft_reset_at = Some(now);
        }

        let keep = HashSet::new();
        let (purged_thumb_jobs, canceled_thumb_subs) = self
            .loader
            .retain_queued_thumbnails_epoch_keys(self.streaming_runtime.stream_epoch, &keep);

        if stage0_log_enabled() {
            log::info!(
                "Stage0Reset | reason=pan action=preview_soft zoom_bucket {}->{} pan_px={:.1} thr_px={:.1} moving={} cooldown={} pending_thumbs={} purged_thumb_jobs={} canceled_thumb_subs={}",
                last_bucket,
                cur_bucket,
                pan_px,
                self.viewport_runtime().preview_soft_reset_pan_delta_px,
                self.viewport_runtime().moving_recently,
                self.viewport_runtime().preview_soft_reset_cooldown_frames,
                pending_thumbs,
                purged_thumb_jobs,
                canceled_thumb_subs
            );
        }
    }

    pub(super) fn maybe_bump_stream_epoch(&mut self) {
        let now = Instant::now();
        let zoom = self.view().zoom;
        let cur_bucket = zoom.max(1e-9).log2().floor() as i32;
        let last_bucket = self
            .streaming_runtime
            .last_epoch_zoom
            .max(1e-9)
            .log2()
            .floor() as i32;
        let pan_px = self.center_pan_delta_since_soft_reset_px();

        // Preview-only soft reset on pan movement:
        // drop stale thumbnail intent/queue without touching tile epoch.
        let pan_cooldown_ready = self
            .viewport_runtime()
            .last_preview_soft_reset_at
            .map(|reset_at| {
                now.saturating_duration_since(reset_at)
                    >= self
                        .viewport_runtime()
                        .preview_soft_reset_cooldown_duration()
            })
            .unwrap_or(true);
        let pan_trigger = self.viewport_runtime().moving_recently
            && pan_cooldown_ready
            && pan_px >= self.viewport_runtime().preview_soft_reset_pan_delta_px
            && (self.has_pending_slots_current() || self.loader.has_pending_work());
        if pan_trigger {
            self.bump_preview_soft_reset(pan_px, cur_bucket, last_bucket);
        }

        // Epoch changes only on zoom bucket transitions while camera is stable.
        // Pan now relies on live reprioritization without epoch resets.
        let zoom_bucket_changed =
            cur_bucket != last_bucket && !self.viewport_runtime().moving_recently;
        if !zoom_bucket_changed {
            return;
        }

        let settle_frames = self.viewport_runtime().zoom_reset_settle_frames.max(1);
        let zoom_stable = self
            .viewport_runtime()
            .last_zoom_changed_at
            .map(|changed_at| {
                now.saturating_duration_since(changed_at)
                    >= self.viewport_runtime().zoom_reset_settle_duration()
            })
            .unwrap_or(true);
        if !zoom_stable {
            return;
        }

        let cooldown_frames = self.viewport_runtime().zoom_reset_cooldown_frames;
        let cooldown_ready = self
            .viewport_runtime()
            .last_zoom_reset_at
            .map(|reset_at| {
                now.saturating_duration_since(reset_at)
                    >= self.viewport_runtime().zoom_reset_cooldown_duration()
            })
            .unwrap_or(true);
        if !cooldown_ready {
            return;
        }

        if stage0_log_enabled() {
            log::info!(
                "Stage0Reset | reason=zoom action=epoch zoom_bucket {}->{} pan_px={:.1} moving={} settle={} cooldown={}",
                last_bucket,
                cur_bucket,
                pan_px,
                self.viewport_runtime().moving_recently,
                settle_frames,
                cooldown_frames,
            );
        }
        self.streaming_runtime.last_epoch_zoom = zoom;
        let frame_count = self.frame_count;
        {
            let runtime = self.viewport_runtime_mut();
            runtime.last_zoom_reset_frame = frame_count;
            runtime.last_zoom_reset_at = Some(now);
        }
        self.bump_stream_epoch_soft();
    }
}
