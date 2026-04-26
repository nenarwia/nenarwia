use super::super::UiRenderable;
use super::state::MenuTexture;
use super::CanvasContextMenuUi;

impl CanvasContextMenuUi {
    pub fn render_panel(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        if !self.open {
            return;
        }

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Canvas Context Menu Pass"),
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

    pub(super) fn update_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &MenuTexture,
    ) {
        if texture.width != self.tex_width || texture.height != self.tex_height {
            let (wgpu_texture, texture_view) =
                super::super::notice_texture::create_texture(device, texture.width, texture.height);
            self.texture = wgpu_texture;
            self.texture_view = texture_view;
            self.tex_width = texture.width;
            self.tex_height = texture.height;
            self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("canvas_context_menu_bind_group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
        }

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &texture.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(texture.width * 4),
                rows_per_image: Some(texture.height),
            },
            wgpu::Extent3d {
                width: texture.width,
                height: texture.height,
                depth_or_array_layers: 1,
            },
        );
    }
}

impl UiRenderable for CanvasContextMenuUi {
    fn render_overlay(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        self.render_panel(encoder, view);
    }
}
