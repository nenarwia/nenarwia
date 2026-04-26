use super::super::UiRenderable;
use super::state::{BurgerTexture, PanelTexture};
use super::SidebarUi;

impl SidebarUi {
    pub fn render_panel(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        if self.open_t <= 0.001 {
            return;
        }

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Sidebar Panel Pass"),
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
        pass.set_bind_group(0, &self.panel_bind_group, &[]);
        pass.set_vertex_buffer(0, self.panel_vertex_buffer.slice(..));
        pass.draw(0..self.panel_vertex_count, 0..1);
    }

    pub fn render_burger(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Sidebar Burger Pass"),
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
        pass.set_bind_group(0, &self.burger_bind_group, &[]);
        pass.set_vertex_buffer(0, self.burger_vertex_buffer.slice(..));
        pass.draw(0..self.burger_vertex_count, 0..1);
    }

    pub(super) fn update_panel_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        panel: &PanelTexture,
    ) {
        if panel.width != self.panel_tex_width || panel.height != self.panel_tex_height {
            let (texture, view) =
                super::super::notice_texture::create_texture(device, panel.width, panel.height);
            self.panel_texture = texture;
            self.panel_texture_view = view;
            self.panel_tex_width = panel.width;
            self.panel_tex_height = panel.height;
            self.panel_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("sidebar_panel_bind_group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.panel_texture_view),
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
                texture: &self.panel_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &panel.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(panel.width * 4),
                rows_per_image: Some(panel.height),
            },
            wgpu::Extent3d {
                width: panel.width,
                height: panel.height,
                depth_or_array_layers: 1,
            },
        );
    }

    pub(super) fn update_burger_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        burger: &BurgerTexture,
    ) {
        if burger.width != self.burger_tex_width || burger.height != self.burger_tex_height {
            let (texture, view) =
                super::super::notice_texture::create_texture(device, burger.width, burger.height);
            self.burger_texture = texture;
            self.burger_texture_view = view;
            self.burger_tex_width = burger.width;
            self.burger_tex_height = burger.height;
            self.burger_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("sidebar_burger_bind_group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.burger_texture_view),
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
                texture: &self.burger_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &burger.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(burger.width * 4),
                rows_per_image: Some(burger.height),
            },
            wgpu::Extent3d {
                width: burger.width,
                height: burger.height,
                depth_or_array_layers: 1,
            },
        );
    }
}

impl UiRenderable for SidebarUi {
    fn render_under_chrome(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        self.render_panel(encoder, view);
    }

    fn render_overlay(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        self.render_burger(encoder, view);
    }
}
