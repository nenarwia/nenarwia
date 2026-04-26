use crate::render::context::state::RenderContext;

impl RenderContext {
    pub(super) fn poll_gpu_feedback(&mut self) {
        if !self.streaming.use_gpu_feedback {
            return;
        }

        let results = match self.gpu_feedback.as_mut() {
            Some(gpu_feedback) => gpu_feedback.collect_ready(self.frame_count),
            None => Vec::new(),
        };
        if results.is_empty() {
            return;
        }

        let summary = crate::render::streaming::feedback::apply_feedback_results(self, results);
        self.feedback.last_ready_frame = self.frame_count;
        self.feedback.overflow_last = summary.overflow;
        self.feedback.latency_last = summary.latency_frames;
        self.feedback.has_results = summary.unique_tiles > 0;
        self.quality_stats.feedback_pages_last = summary.unique_tiles;
        self.quality_stats.feedback_overflow_last = if summary.overflow { 1 } else { 0 };
        self.quality_stats.feedback_latency_last = summary.latency_frames;
    }
}
