use std::time::Duration;

#[derive(Clone, Copy, Debug, Default)]
pub struct Stage0Metrics {
    frames: u32,
    visibility_ms_sum: f32,
    scheduler_ms_sum: f32,
    upload_ms_sum: f32,
    evicted_pages: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Stage0Snapshot {
    pub frames: u32,
    pub visibility_ms_avg: f32,
    pub scheduler_ms_avg: f32,
    pub upload_ms_avg: f32,
    pub evicted_pages: u32,
}

impl Stage0Metrics {
    pub fn on_frame(&mut self) {
        self.frames = self.frames.saturating_add(1);
    }

    pub fn record_visibility(&mut self, elapsed: Duration) {
        self.visibility_ms_sum += elapsed.as_secs_f32() * 1000.0;
    }

    pub fn record_scheduler(&mut self, elapsed: Duration) {
        self.scheduler_ms_sum += elapsed.as_secs_f32() * 1000.0;
    }

    pub fn record_upload(&mut self, elapsed: Duration) {
        self.upload_ms_sum += elapsed.as_secs_f32() * 1000.0;
    }

    pub fn record_evicted_pages(&mut self, count: u32) {
        self.evicted_pages = self.evicted_pages.saturating_add(count);
    }

    pub fn take_snapshot(&mut self) -> Stage0Snapshot {
        let frames = self.frames;
        let denom = if frames > 0 { frames as f32 } else { 1.0 };
        let snapshot = Stage0Snapshot {
            frames,
            visibility_ms_avg: self.visibility_ms_sum / denom,
            scheduler_ms_avg: self.scheduler_ms_sum / denom,
            upload_ms_avg: self.upload_ms_sum / denom,
            evicted_pages: self.evicted_pages,
        };
        *self = Stage0Metrics::default();
        snapshot
    }
}
