use std::collections::HashSet;

use winit::dpi::{PhysicalPosition, PhysicalSize};

use crate::core::color::MissingCodecKind;

use super::bind_groups::create_texture_sampler_bind_group;
use super::notice_texture::{
    build_notice_texture, create_texture, layout_max_width, point_in_rect, NoticeTexture,
};
use super::{CodecNoticeUi, UiRenderable, UiVertex, UI_MARGIN_PX};

impl CodecNoticeUi {
    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_size: PhysicalSize<u32>,
        kinds: Vec<MissingCodecKind>,
    ) {
        if kinds.is_empty() {
            self.visible = false;
            return;
        }

        let current_set: HashSet<MissingCodecKind> = kinds.iter().copied().collect();
        let has_new = current_set
            .iter()
            .any(|k| !self.dismissed_kinds.contains(k));
        if self.dismissed && !has_new {
            self.visible = false;
            return;
        }
        if has_new {
            self.dismissed = false;
        }

        self.visible = true;

        let layout_width = layout_max_width(surface_size);
        if kinds != self.active_kinds || layout_width != self.last_layout_width {
            self.active_kinds = kinds;
            if let Some(notice) = build_notice_texture(&self.active_kinds, layout_width) {
                self.update_texture(device, queue, &notice);
                self.close_rect_local_px = notice.close_rect;
                self.last_layout_width = layout_width;
            } else {
                self.visible = false;
                return;
            }
        }

        if self.visible {
            self.update_vertices(queue, surface_size);
        }
    }

    pub fn render(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        if !self.visible {
            return;
        }

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("UI Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            multiview_mask: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
    }

    pub fn handle_click(&mut self, pos: PhysicalPosition<f64>) -> bool {
        if !self.visible {
            return false;
        }
        let Some(box_rect) = self.box_rect_px else {
            return false;
        };
        let (x, y) = (pos.x as f32, pos.y as f32);
        if !point_in_rect(x, y, box_rect) {
            return false;
        }

        if let Some(close_rect) = self.close_rect_px {
            if point_in_rect(x, y, close_rect) {
                self.dismissed = true;
                self.dismissed_kinds.clear();
                self.dismissed_kinds
                    .extend(self.active_kinds.iter().copied());
                self.visible = false;
            }
        }

        true
    }

    fn update_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        notice: &NoticeTexture,
    ) {
        if notice.width != self.tex_width || notice.height != self.tex_height {
            let (texture, view) = create_texture(device, notice.width, notice.height);
            self.texture = texture;
            self.texture_view = view;
            self.tex_width = notice.width;
            self.tex_height = notice.height;
            self.bind_group = create_texture_sampler_bind_group(
                device,
                Some("ui_bind_group"),
                &self.bind_group_layout,
                &self.texture_view,
                &self.sampler,
            );
        }

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &notice.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(notice.width * 4),
                rows_per_image: Some(notice.height),
            },
            wgpu::Extent3d {
                width: notice.width,
                height: notice.height,
                depth_or_array_layers: 1,
            },
        );
    }

    fn update_vertices(&mut self, queue: &wgpu::Queue, surface_size: PhysicalSize<u32>) {
        if surface_size.width == 0 || surface_size.height == 0 {
            return;
        }

        let box_w = self.tex_width as f32;
        let box_h = self.tex_height as f32;

        let margin = UI_MARGIN_PX as f32;
        let mut x = margin;
        let mut y = surface_size.height as f32 - margin - box_h;
        if y < margin {
            y = margin;
        }
        if x + box_w > surface_size.width as f32 - margin {
            x = (surface_size.width as f32 - margin - box_w).max(margin);
        }

        self.box_rect_px = Some([x, y, box_w, box_h]);
        let close = self.close_rect_local_px;
        self.close_rect_px = Some([
            x + close[0] as f32,
            y + close[1] as f32,
            close[2] as f32,
            close[3] as f32,
        ]);

        let left = x / surface_size.width as f32 * 2.0 - 1.0;
        let right = (x + box_w) / surface_size.width as f32 * 2.0 - 1.0;
        let top = 1.0 - (y / surface_size.height as f32 * 2.0);
        let bottom = 1.0 - ((y + box_h) / surface_size.height as f32 * 2.0);

        let verts = [
            UiVertex {
                position: [left, bottom],
                uv: [0.0, 1.0],
            },
            UiVertex {
                position: [right, bottom],
                uv: [1.0, 1.0],
            },
            UiVertex {
                position: [right, top],
                uv: [1.0, 0.0],
            },
            UiVertex {
                position: [left, bottom],
                uv: [0.0, 1.0],
            },
            UiVertex {
                position: [right, top],
                uv: [1.0, 0.0],
            },
            UiVertex {
                position: [left, top],
                uv: [0.0, 0.0],
            },
        ];

        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&verts));
    }
}

impl UiRenderable for CodecNoticeUi {
    fn render_overlay(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        self.render(encoder, view);
    }
}
