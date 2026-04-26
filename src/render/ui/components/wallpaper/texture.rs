use crate::render::ui::WallpaperUi;

use super::super::bind_groups::create_texture_sampler_uniform_bind_group;
use super::super::notice_texture::create_texture;
use super::state::WallpaperParams;

impl WallpaperUi {
    pub fn set_from_rgba(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) {
        if width == 0 || height == 0 {
            log::warn!(
                "Ignoring wallpaper update with invalid size: {}x{}",
                width,
                height
            );
            return;
        }

        let required_len = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4);
        if pixels.len() < required_len {
            log::warn!(
                "Ignoring wallpaper update: buffer too small (have {}, need {})",
                pixels.len(),
                required_len
            );
            return;
        }

        self.update_texture(device, queue, width, height, pixels);
        self.enabled = true;
    }

    pub fn set_dimming(&mut self, queue: &wgpu::Queue, dim_amount: f32) {
        let dim_amount = dim_amount.clamp(0.0, 1.0);
        if (self.dim_amount - dim_amount).abs() <= 0.0001 {
            return;
        }
        self.dim_amount = dim_amount;
        let params = WallpaperParams {
            dim: [dim_amount, 0.0, 0.0, 0.0],
        };
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));
    }

    fn update_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) {
        let (texture, texture_view) = create_texture(device, width, height);
        self.texture = texture;
        self.texture_view = texture_view;
        self.image_width = width.max(1);
        self.image_height = height.max(1);
        self.last_surface_width = 0;
        self.last_surface_height = 0;
        self.bind_group = create_texture_sampler_uniform_bind_group(
            device,
            Some("wallpaper_bind_group"),
            &self.bind_group_layout,
            &self.texture_view,
            &self.sampler,
            &self.params_buffer,
        );

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }
}
