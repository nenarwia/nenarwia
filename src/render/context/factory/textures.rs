use crate::render::atlas::MultiTierAtlas;
use crate::render::cache::{PageDirectory, PageTable, PhysicalCache, TileFormat};
use crate::render::context::budget::CacheConfig;

pub struct TextureSystems {
    pub atlas: MultiTierAtlas,
    pub tile_cache: PhysicalCache,
    pub page_table: PageTable,
    pub page_directory: PageDirectory,
}

pub fn create_texture_systems(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    cfg: &CacheConfig,
    max_dim: u32,
    tile_format: TileFormat,
) -> TextureSystems {
    // 1) Thumbnail atlas
    let atlas = MultiTierAtlas::new(
        device,
        queue,
        [
            cfg.thumb_atlas_dim_32,
            cfg.thumb_atlas_dim_64,
            cfg.thumb_atlas_dim_128,
            cfg.thumb_atlas_dim_256,
            cfg.thumb_atlas_dim_512,
        ],
        max_dim,
    );

    // 2) Physical tile cache
    let tile_cache = PhysicalCache::new(device, cfg.tile_cache_dim, max_dim, tile_format);
    let page_table = PageTable::new(tile_cache.total_slots);

    // 3) Page Directory
    let page_directory = PageDirectory::new(device);

    TextureSystems {
        atlas,
        tile_cache,
        page_table,
        page_directory,
    }
}
