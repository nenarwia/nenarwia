use super::constants::TILE_PHYSICAL_SIZE;

#[derive(Clone, Copy, Debug)]
pub struct TileFormat {
    pub format: wgpu::TextureFormat,
    pub block_dim: u32,
    pub block_bytes: u32,
}

impl TileFormat {
    pub fn rgba8_srgb() -> Self {
        Self {
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            block_dim: 1,
            block_bytes: 4,
        }
    }
}

pub struct PhysicalCache {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,

    pub cache_dim: u32,
    pub cols: u32,
    pub total_slots: u32,
    pub format: TileFormat,
}

impl PhysicalCache {
    pub fn new(
        device: &wgpu::Device,
        requested_cache_dim: u32,
        max_dim: u32,
        format: TileFormat,
    ) -> Self {
        let mut cache_dim = requested_cache_dim.max(TILE_PHYSICAL_SIZE).min(max_dim);
        cache_dim = (cache_dim / TILE_PHYSICAL_SIZE) * TILE_PHYSICAL_SIZE;
        cache_dim = cache_dim.max(TILE_PHYSICAL_SIZE);

        let block = format.block_dim.max(1);
        cache_dim = (cache_dim / block) * block;

        let cols = cache_dim / TILE_PHYSICAL_SIZE;
        let total_slots = cols * cols;

        let size = wgpu::Extent3d {
            width: cache_dim,
            height: cache_dim,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Tile Cache Texture (Physical)"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: format.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture,
            view,
            cache_dim,
            cols,
            total_slots,
            format,
        }
    }

    // Upload a single physical tile (logical tile + halo) into a specific cache slot.
    pub fn upload_tile(&self, queue: &wgpu::Queue, slot: u32, data: &[u8]) {
        let col = slot % self.cols;
        let row = slot / self.cols;

        if !self.payload_len_matches(data) {
            return;
        }

        let x = col * TILE_PHYSICAL_SIZE;
        let y = row * TILE_PHYSICAL_SIZE;
        let blocks_x = (TILE_PHYSICAL_SIZE / self.format.block_dim).max(1);
        let blocks_y = (TILE_PHYSICAL_SIZE / self.format.block_dim).max(1);
        let bytes_per_row = self.format.block_bytes * blocks_x;

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(blocks_y),
            },
            wgpu::Extent3d {
                width: TILE_PHYSICAL_SIZE,
                height: TILE_PHYSICAL_SIZE,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn payload_len_matches(&self, data: &[u8]) -> bool {
        let blocks_x = (TILE_PHYSICAL_SIZE / self.format.block_dim).max(1);
        let blocks_y = (TILE_PHYSICAL_SIZE / self.format.block_dim).max(1);
        let expected = (self.format.block_bytes * blocks_x * blocks_y) as usize;
        data.len() >= expected
    }

    pub fn bytes_per_tile(&self) -> u64 {
        let blocks_x = (TILE_PHYSICAL_SIZE / self.format.block_dim).max(1);
        let blocks_y = (TILE_PHYSICAL_SIZE / self.format.block_dim).max(1);
        (blocks_x * blocks_y * self.format.block_bytes) as u64
    }
}
