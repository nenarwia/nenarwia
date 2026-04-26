use super::region::{PtRegion, PT_TEXTURE_SIZE};
pub struct DirectoryGpu {
    pub texture: wgpu::Texture,
}

impl DirectoryGpu {
    pub fn new(device: &wgpu::Device) -> Self {
        let size = wgpu::Extent3d {
            width: PT_TEXTURE_SIZE,
            height: PT_TEXTURE_SIZE,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Page Directory Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        Self { texture }
    }

    pub fn clear_region(&self, queue: &wgpu::Queue, region: PtRegion) {
        if region.w == 0 || region.h == 0 {
            return;
        }
        let row_bytes = 4 * region.w;
        let bytes_per_row = align_up(row_bytes, 256);
        let zero_data = vec![0u8; (bytes_per_row * region.h) as usize];

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: region.x,
                    y: region.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &zero_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(region.h),
            },
            wgpu::Extent3d {
                width: region.w,
                height: region.h,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn reset_all(&self, queue: &wgpu::Queue) {
        let bytes_per_row = 4 * PT_TEXTURE_SIZE;
        let zero_data = vec![0u8; (bytes_per_row * PT_TEXTURE_SIZE) as usize];

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &zero_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(PT_TEXTURE_SIZE),
            },
            wgpu::Extent3d {
                width: PT_TEXTURE_SIZE,
                height: PT_TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn update_entry(
        &self,
        queue: &wgpu::Queue,
        region: PtRegion,
        tile_x: u32,
        tile_y: u32,
        slot: Option<u32>,
    ) {
        if tile_x >= region.w || tile_y >= region.h {
            return;
        }

        let pixel_x = region.x + tile_x;
        let pixel_y = region.y + tile_y;

        let (r, g, b, a) = if let Some(slot) = slot {
            ((slot & 0xFF) as u8, ((slot >> 8) & 0xFF) as u8, 0u8, 255u8)
        } else {
            (0u8, 0u8, 0u8, 0u8)
        };

        let mut row = [0u8; 256];
        row[0] = r;
        row[1] = g;
        row[2] = b;
        row[3] = a;

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: pixel_x,
                    y: pixel_y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &row,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
    }
}

fn align_up(value: u32, align: u32) -> u32 {
    if align == 0 {
        return value;
    }
    value.div_ceil(align) * align
}
