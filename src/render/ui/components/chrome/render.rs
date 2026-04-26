use super::super::bind_groups::create_texture_sampler_bind_group;
use super::super::UiRenderable;
use super::state::ChromeTexture;
use super::WindowChromeUi;

impl WindowChromeUi {
    pub fn render(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        if self.window_rect_px.is_none() {
            return;
        }

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Window Chrome Pass"),
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
        chrome: &ChromeTexture,
    ) {
        if chrome.width != self.tex_width || chrome.height != self.tex_height {
            let (texture, view) =
                super::super::notice_texture::create_texture(device, chrome.width, chrome.height);
            self.texture = texture;
            self.texture_view = view;
            self.tex_width = chrome.width;
            self.tex_height = chrome.height;
            self.bind_group = create_texture_sampler_bind_group(
                device,
                Some("window_chrome_bind_group"),
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
            &chrome.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(chrome.width * 4),
                rows_per_image: Some(chrome.height),
            },
            wgpu::Extent3d {
                width: chrome.width,
                height: chrome.height,
                depth_or_array_layers: 1,
            },
        );
    }
}

impl UiRenderable for WindowChromeUi {
    fn render_overlay(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        self.render(encoder, view);
    }
}
