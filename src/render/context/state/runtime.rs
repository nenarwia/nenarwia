use std::time::{Duration, Instant};

use super::{QualityStats, RenderContext, Stage0Snapshot};
use crate::spatial::navigation::ViewRuntime;
use crate::spatial::view::{ViewMetrics, ViewState};
impl RenderContext {
    pub(crate) fn duration_for_reference_frames(frames: u64) -> Duration {
        Duration::from_secs_f64(frames as f64 / 60.0)
    }

    pub fn view(&self) -> ViewState {
        self.viewport.current()
    }

    pub fn view_metrics(&self) -> ViewMetrics {
        self.viewport.metrics()
    }

    pub fn mark_redraw_pending(&mut self) {
        self.pending_redraw = true;
    }

    pub fn has_pending_redraw(&self) -> bool {
        self.pending_redraw
    }

    pub fn clear_pending_redraw(&mut self) {
        self.pending_redraw = false;
    }

    pub fn clear_continuous_redraw_schedule(&mut self) {
        self.next_continuous_redraw_at = None;
    }

    pub fn viewport_runtime(&self) -> &ViewRuntime {
        self.viewport.runtime()
    }

    pub fn viewport_runtime_mut(&mut self) -> &mut ViewRuntime {
        self.viewport.runtime_mut()
    }

    pub fn fit_scene_immediate(&mut self, padding_factor: f64) -> bool {
        let Some(bounds) = self.scene.bounds() else {
            return false;
        };
        self.viewport.set_content_bounds(Some(bounds));
        self.viewport
            .fit_bounds(bounds, padding_factor, Instant::now())
    }

    pub(crate) fn compute_backdrop_blur_size(
        full: winit::dpi::PhysicalSize<u32>,
    ) -> winit::dpi::PhysicalSize<u32> {
        // Quarter-res backdrop blur approximates CSS blur(20px) with a short kernel.
        winit::dpi::PhysicalSize::new(
            ((full.width + 3) / 4).max(1),
            ((full.height + 3) / 4).max(1),
        )
    }

    pub(crate) fn create_scene_color_target(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        size: winit::dpi::PhysicalSize<u32>,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scene_color_target"),
            size: wgpu::Extent3d {
                width: size.width.max(1),
                height: size.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.gpu.resize(new_size);
        self.viewport.apply_intent(
            crate::spatial::navigation::ViewportIntent::ResizeSurface(new_size),
            Instant::now(),
        );
        let (scene_texture, scene_view) =
            Self::create_scene_color_target(&self.gpu.device, self.gpu.config.format, new_size);
        self.scene_color_texture = scene_texture;
        self.scene_color_view = scene_view;

        self.backdrop_blur_size = Self::compute_backdrop_blur_size(new_size);
        let blur_fmt = crate::render::ui::backdrop::BLUR_TEXTURE_FORMAT;
        let (blur_a_texture, blur_a_view) =
            Self::create_scene_color_target(&self.gpu.device, blur_fmt, self.backdrop_blur_size);
        let (blur_b_texture, blur_b_view) =
            Self::create_scene_color_target(&self.gpu.device, blur_fmt, self.backdrop_blur_size);
        self.backdrop_blur_a_texture = blur_a_texture;
        self.backdrop_blur_a_view = blur_a_view;
        self.backdrop_blur_b_texture = blur_b_texture;
        self.backdrop_blur_b_view = blur_b_view;
        self.backdrop_blur.rebuild_bind_groups(
            &self.gpu.device,
            &self.scene_color_view,
            &self.backdrop_blur_a_view,
            &self.backdrop_blur_b_view,
        );
    }

    pub fn has_pending_canvas_media_slots_current(&self) -> bool {
        self.streaming_runtime
            .has_pending_canvas_media_slots_current()
    }

    pub fn has_pending_slots_current(&self) -> bool {
        self.streaming_runtime.has_pending_slots_current()
    }

    pub(crate) fn clear_quality_visibility_tracking(&mut self) {
        self.quality_visible_since.clear();
        self.quality_last_cleanup_frame = self.frame_count;
    }

    pub(crate) fn clear_draw_assembly_state(&mut self) {
        self.draw_assembly.clear_draw_instances();
    }

    pub(crate) fn reset_slot_backdrop_state(&mut self) {
        self.draw_assembly.reset_slot_backdrop();
    }

    pub(crate) fn mark_slot_backdrop_dirty(&mut self) {
        self.draw_assembly.mark_slot_backdrop_dirty();
    }

    pub fn take_quality_stats(&mut self) -> QualityStats {
        let stats = self.quality_stats;
        self.quality_stats = QualityStats::default();
        stats
    }

    pub fn take_stage0_metrics(&mut self) -> Stage0Snapshot {
        self.stage0_metrics.take_snapshot()
    }

    pub fn needs_camera_settle_redraw(&self) -> bool {
        self.viewport_runtime()
            .needs_settle_redraw(self.view(), self.streaming_runtime.last_epoch_zoom)
    }
}
