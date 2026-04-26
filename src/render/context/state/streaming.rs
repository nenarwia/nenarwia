#[derive(Clone, Debug)]
pub struct StreamingConfig {
    /// How many extra tiles to request around the visible viewport.
    pub prefetch_radius_tiles: u32,
    /// Hard cap on how many new tile requests we issue per 60 Hz reference frame.
    pub max_canvas_media_slot_requests_per_frame: usize,
    /// CPU budget for tile generation (ms per 60 Hz reference frame).
    pub canvas_media_slot_cpu_budget_ms: f32,
    /// Max number of in-flight tile jobs.
    pub max_inflight_canvas_media_slots: usize,
    /// Hard cap on how many new thumbnail requests we issue per 60 Hz reference frame.
    pub max_thumb_requests_per_frame: usize,
    /// Guaranteed minimum visible preview coverage requests per 60 Hz reference frame when idle.
    pub min_visible_previews_per_frame: usize,
    /// Guaranteed minimum visible preview coverage requests per 60 Hz reference frame while moving.
    pub min_visible_previews_moving_per_frame: usize,
    /// Hard cap on preview requests per 60 Hz reference frame while camera is moving.
    pub max_preview_requests_moving_per_frame: usize,
    /// Hard cap on queued tile requests (prefetch may be dropped first).
    pub max_canvas_media_slot_queue_len: usize,
    /// CPU budget for upload processing (ms per 60 Hz reference frame).
    pub cpu_budget_ms_upload: u32,
    /// Hard cap on upload responses per 60 Hz reference frame.
    pub max_uploads_per_frame: usize,
    /// Minimum visible tiles to dispatch per 60 Hz reference frame.
    pub min_visible_canvas_media_slots_per_frame: usize,
    /// Enable GPU feedback path (tile requests generated on GPU).
    pub use_gpu_feedback: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            prefetch_radius_tiles: 1,
            max_canvas_media_slot_requests_per_frame: 256,
            canvas_media_slot_cpu_budget_ms: 3.0,
            max_inflight_canvas_media_slots: 4,
            max_thumb_requests_per_frame: 32,
            min_visible_previews_per_frame: 12,
            min_visible_previews_moving_per_frame: 12,
            max_preview_requests_moving_per_frame: 32,
            max_canvas_media_slot_queue_len: 100_000,
            cpu_budget_ms_upload: 2,
            max_uploads_per_frame: 200,
            min_visible_canvas_media_slots_per_frame: 8,
            use_gpu_feedback: true,
        }
    }
}
