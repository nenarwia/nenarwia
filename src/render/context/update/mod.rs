pub mod budget;
pub(crate) mod streaming;
pub mod uniforms;
pub mod upload;
pub mod visibility;

mod dialogs;
mod epoch;
mod feedback;
mod wallpaper_preview;

use std::time::{Duration, Instant};

use crate::render::context::state::RenderContext;
use crate::render::ui::{UiUpdatable, UiUpdateCtx};

impl RenderContext {
    pub fn update_at(&mut self, now: Instant) {
        self.frame_count += 1;
        self.quality_stats.reset_frame_counters();
        self.stage0_metrics.on_frame();
        let frame_dt = self
            .last_update_at
            .map(|last| now.saturating_duration_since(last))
            .filter(|dt| *dt > Duration::ZERO)
            .unwrap_or_else(|| Self::duration_for_reference_frames(1));
        self.last_update_at = Some(now);

        // Drive async GPU map callbacks only when GPU feedback readbacks are active.
        if self.streaming.use_gpu_feedback {
            let _ = self.gpu.device.poll(wgpu::PollType::Poll);
        }

        self.flush_stale_system_file_drop();
        self.poll_document_scan_results();
        self.poll_trash_delete_result();
        self.poll_canvas_import_dialog_result();
        self.poll_empty_slot_fill_dialog_result();
        self.poll_wallpaper_dialog_result();
        self.poll_wallpaper_preview_result();
        self.poll_wallpaper_apply_result();
        self.wallpaper_preview_ui
            .poll_blur_job(&self.gpu.device, &self.gpu.queue, self.gpu.size);
        self.wallpaper_preview_ui
            .advance_blur_animation(&self.gpu.queue);

        self.poll_gpu_feedback();

        // 0) VRAM-aware budgeting.
        budget::maybe_update_budget(self);

        let view = self.view();
        let view_metrics = self.view_metrics();
        let frame_count = self.frame_count;
        let runtime_update =
            self.viewport_runtime_mut()
                .update(view, view_metrics, frame_count, frame_dt, now);
        if runtime_update.pan_changed {
            // Slot backdrops are assembled in camera-local space, so a viewport pan
            // changes their vertex data even when scene content itself is unchanged.
            self.mark_slot_backdrop_dirty();
        }
        self.maybe_bump_stream_epoch();

        streaming::prepare_frame(self, frame_dt);

        // 1. Process visible set (CPU R-tree).
        let vis_start = Instant::now();
        visibility::process_visible(self);
        self.stage0_metrics.record_visibility(vis_start.elapsed());

        // 2. Plan preview/tile/video work from the committed view.
        streaming::process_committed_view(self);

        // 3. Drain queued canvas media slot requests (visible first).
        let sched_start = Instant::now();
        streaming::drain_queued_requests(self);
        self.stage0_metrics.record_scheduler(sched_start.elapsed());

        // 4. Upload loaded images.
        let upload_start = Instant::now();
        upload::process_loaded_images(self);
        self.stage0_metrics.record_upload(upload_start.elapsed());
        let coverage_uploaded = self.quality_stats.preview_upload_applied_coverage_last as f32;
        let reference_frames = (frame_dt.as_secs_f32() * 60.0).max(0.001);
        let coverage_uploaded_reference = coverage_uploaded / reference_frames;
        let ema_alpha = reference_frame_alpha(
            if self.viewport_runtime().moving_recently {
                0.35
            } else {
                0.20
            },
            frame_dt,
        );
        self.streaming_runtime.preview.coverage_upload_ema =
            self.streaming_runtime.preview.coverage_upload_ema * (1.0 - ema_alpha)
                + coverage_uploaded_reference * ema_alpha;

        // 5. Update uniforms.
        uniforms::update_camera_uniforms(self);

        // 6. Refresh visible GPU buffer.
        visibility::update_slot_backdrop_buffer(self);
        visibility::update_visible_buffer(self);

        // 7. UI notices (missing codecs, etc.).
        self.wallpaper_ui
            .update_layout(&self.gpu.queue, self.gpu.size);
        self.wallpaper_preview_ui
            .update_layout(&self.gpu.queue, self.gpu.size);
        self.sync_window_chrome_tabs();
        let ui_ctx = UiUpdateCtx {
            device: &self.gpu.device,
            queue: &self.gpu.queue,
            surface_size: self.gpu.size,
            window_maximized: self.window.is_maximized() || self.window.fullscreen().is_some(),
            vsync_enabled: self.frame_pacing_mode.is_vsync(),
            graphics_backend_preference: self.graphics_backend_preference,
            debug_slot_backdrop_enabled: self.debug_slot_backdrop_enabled,
        };
        self.window_chrome.update_ui(ui_ctx);
        self.sidebar_ui.update_ui(ui_ctx);
        self.canvas_context_menu.update_ui(ui_ctx);
        self.codec_notice.update(
            &self.gpu.device,
            &self.gpu.queue,
            self.gpu.size,
            crate::core::color::missing_codec_kinds(),
        );
    }
}

fn reference_frame_alpha(alpha_reference_frame: f32, frame_dt: Duration) -> f32 {
    let dt_reference_frames = frame_dt.as_secs_f32().max(0.0) * 60.0;
    1.0 - (1.0 - alpha_reference_frame.clamp(0.0, 0.999_999)).powf(dt_reference_frames)
}
