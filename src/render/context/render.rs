use super::state::RenderContext;
use crate::render::streaming::feedback::FeedbackEncodeInput;
use crate::render::ui::UiRenderable;
use std::sync::OnceLock;

// Surface format is sRGB; use linear values that map to sRGB hex #F3F3F5 on screen.
const CANVAS_BG_LINEAR_FROM_SRGB_F3: f64 = 0.8962693533742664;
const CANVAS_BG_LINEAR_FROM_SRGB_F5: f64 = 0.9130986517934192;

fn backdrop_blur_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        let disable = std::env::var("CANVAS_DISABLE_BACKDROP_BLUR")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if matches!(disable.as_str(), "1" | "true" | "yes" | "on") {
            return false;
        }

        let enable = std::env::var("CANVAS_ENABLE_BACKDROP_BLUR")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if enable.is_empty() {
            return true;
        }
        matches!(enable.as_str(), "1" | "true" | "yes" | "on")
    })
}

impl RenderContext {
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let use_backdrop_blur = backdrop_blur_enabled();
        let output = self.gpu.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let scene_target_view = if use_backdrop_blur {
            &self.scene_color_view
        } else {
            &view
        };
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Frame Encoder"),
            });

        if self.streaming.use_gpu_feedback {
            if let Some(feedback) = self.gpu_feedback.as_mut() {
                feedback.encode(FeedbackEncodeInput {
                    queue: &self.gpu.queue,
                    encoder: &mut encoder,
                    camera_bg: &self.camera_bind_group,
                    instance_buffer: &self.draw_assembly.visible_buffer,
                    instance_count: self.draw_assembly.visible_count,
                    feedback_instance_bg: &self.draw_assembly.feedback_instance_bind_group,
                    feedback_collect_buf_bg: &self.draw_assembly.feedback_collect_buf_bind_group,
                    frame_index: self.frame_count,
                });
            }
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: scene_target_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: CANVAS_BG_LINEAR_FROM_SRGB_F3,
                            g: CANVAS_BG_LINEAR_FROM_SRGB_F3,
                            b: CANVAS_BG_LINEAR_FROM_SRGB_F5,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                multiview_mask: None,
                timestamp_writes: None,
            });
            self.wallpaper_ui.render(&mut pass);
            if self.draw_assembly.slot_backdrop_count > 0 {
                pass.set_pipeline(&self.slot_backdrop_pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_vertex_buffer(0, self.draw_assembly.slot_backdrop_buffer.slice(..));
                pass.draw(0..6, 0..self.draw_assembly.slot_backdrop_count);
            }
            if self.draw_assembly.visible_count > 0 {
                pass.set_pipeline(&self.render_pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_bind_group(1, &self.diffuse_bind_group, &[]);
                pass.set_vertex_buffer(0, self.draw_assembly.visible_buffer.slice(..));
                pass.draw(0..6, 0..self.draw_assembly.visible_count);
            }
        }

        if use_backdrop_blur {
            let mut overlay_blur_rects = Vec::with_capacity(3);
            let left_panel_blur_width = self.sidebar_ui.blur_width_px();
            if left_panel_blur_width > 0.5 {
                overlay_blur_rects.push([
                    0.0,
                    0.0,
                    left_panel_blur_width,
                    self.gpu.size.height as f32,
                ]);
            }
            if let Some(rect) = self.canvas_context_menu.blur_rect_px() {
                overlay_blur_rects.push(rect);
            }

            self.backdrop_blur.render(
                &self.gpu.queue,
                &mut encoder,
                &view,
                &self.backdrop_blur_a_view,
                &self.backdrop_blur_b_view,
                self.gpu.size,
                self.backdrop_blur_size,
                overlay_blur_rects.as_slice(),
            );
        }
        self.sidebar_ui.render_under_chrome(&mut encoder, &view);
        self.window_chrome.render_overlay(&mut encoder, &view);
        self.sidebar_ui.render_overlay(&mut encoder, &view);
        self.canvas_context_menu.render_overlay(&mut encoder, &view);
        self.codec_notice.render_overlay(&mut encoder, &view);
        self.wallpaper_preview_ui
            .render_overlay(&mut encoder, &view);

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        if self.streaming.use_gpu_feedback {
            if let Some(feedback) = self.gpu_feedback.as_mut() {
                feedback.request_maps();
            }
        }
        // Feedback readback is handled during update via map_async polling.
        output.present();
        Ok(())
    }
}
