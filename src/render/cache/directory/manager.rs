use super::allocator::FreeRectAllocator;
use super::gpu::DirectoryGpu;
use super::region::{PtRegion, PT_TEXTURE_SIZE};
use std::collections::{HashMap, VecDeque};

#[derive(Clone, Debug)]
pub struct EnsureRegionResult {
    pub region: Option<PtRegion>,
    /// (asset_key, lod, old_region)
    pub evicted: Vec<(u64, u8, PtRegion)>,
}

pub struct PageDirectory {
    gpu: DirectoryGpu,
    pub view: wgpu::TextureView,

    // (asset_key, lod) -> region
    regions: HashMap<(u64, u8), PtRegion>,
    alloc: FreeRectAllocator,

    // LRU state
    lru_queue: VecDeque<((u64, u8), u64)>,
    last_used: HashMap<(u64, u8), u64>,
    usage_counter: u64,
}

impl PageDirectory {
    pub fn new(device: &wgpu::Device) -> Self {
        let gpu = DirectoryGpu::new(device);
        let view_clone = gpu
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            gpu,
            view: view_clone,
            regions: HashMap::new(),
            alloc: FreeRectAllocator::new_full(),
            lru_queue: VecDeque::new(),
            last_used: HashMap::new(),
            usage_counter: 0,
        }
    }

    pub fn get_region(&self, asset_key: u64, lod: u8) -> Option<PtRegion> {
        self.regions.get(&(asset_key, lod)).copied()
    }

    pub fn invalidate_asset(&mut self, queue: &wgpu::Queue, asset_key: u64) -> Vec<(u8, PtRegion)> {
        let victims: Vec<(u8, PtRegion)> = self
            .regions
            .iter()
            .filter_map(|(&(key_asset, lod), &region)| {
                (key_asset == asset_key).then_some((lod, region))
            })
            .collect();

        for (lod, region) in victims.iter().copied() {
            self.regions.remove(&(asset_key, lod));
            self.last_used.remove(&(asset_key, lod));
            self.gpu.clear_region(queue, region);
            self.alloc.free_rect(region);
        }

        victims
    }

    fn touch(&mut self, asset_key: u64, lod: u8) {
        let key = (asset_key, lod);
        if self.regions.contains_key(&key) {
            self.usage_counter = self.usage_counter.wrapping_add(1);
            let stamp = self.usage_counter;
            self.last_used.insert(key, stamp);
            self.lru_queue.push_back((key, stamp));
        }
    }

    fn evict_one_lru(&mut self, queue: &wgpu::Queue) -> Option<(u64, u8, PtRegion)> {
        while let Some(((asset_key, lod), stamp)) = self.lru_queue.pop_front() {
            if self.last_used.get(&(asset_key, lod)).copied() != Some(stamp) {
                continue;
            }

            if let Some(region) = self.regions.remove(&(asset_key, lod)) {
                self.last_used.remove(&(asset_key, lod));
                self.gpu.clear_region(queue, region);
                self.alloc.free_rect(region);

                return Some((asset_key, lod, region));
            }
        }
        None
    }

    pub fn ensure_region(
        &mut self,
        queue: &wgpu::Queue,
        asset_key: u64,
        lod: u8,
        tiles_x: u32,
        tiles_y: u32,
    ) -> EnsureRegionResult {
        let mut evicted: Vec<(u64, u8, PtRegion)> = Vec::new();
        let key = (asset_key, lod);

        if tiles_x == 0 || tiles_y == 0 {
            return EnsureRegionResult {
                region: None,
                evicted,
            };
        }

        // Page directory stores one texel per virtual tile. Oversized layouts cannot be represented.
        if tiles_x > PT_TEXTURE_SIZE || tiles_y > PT_TEXTURE_SIZE {
            if let Some(old) = self.regions.remove(&key) {
                self.last_used.remove(&key);
                self.gpu.clear_region(queue, old);
                self.alloc.free_rect(old);
                evicted.push((asset_key, lod, old));
            }
            return EnsureRegionResult {
                region: None,
                evicted,
            };
        }

        let w = tiles_x;
        let h = tiles_y;

        if let Some(existing) = self.regions.get(&key).copied() {
            if existing.w >= w && existing.h >= h {
                self.touch(asset_key, lod);
                return EnsureRegionResult {
                    region: Some(existing),
                    evicted,
                };
            }
            if let Some(old) = self.regions.remove(&key) {
                self.last_used.remove(&key);
                self.gpu.clear_region(queue, old);
                self.alloc.free_rect(old);
                evicted.push((asset_key, lod, old));
            }
        }

        let mut region = self.alloc.alloc(w, h);
        while region.is_none() {
            let Some(v) = self.evict_one_lru(queue) else {
                break;
            };
            evicted.push(v);
            region = self.alloc.alloc(w, h);
        }

        let Some(region) = region else {
            return EnsureRegionResult {
                region: None,
                evicted,
            };
        };

        self.gpu.clear_region(queue, region);
        self.regions.insert(key, region);
        self.touch(asset_key, lod);

        EnsureRegionResult {
            region: Some(region),
            evicted,
        }
    }

    pub fn reset_all(&mut self, queue: &wgpu::Queue) {
        self.regions.clear();
        self.alloc.reset_full();
        self.lru_queue.clear();
        self.last_used.clear();
        self.usage_counter = 0;
        self.gpu.reset_all(queue);
    }

    pub fn update_entry(
        &self,
        queue: &wgpu::Queue,
        region: PtRegion,
        tile_x: u32,
        tile_y: u32,
        slot: Option<u32>,
    ) {
        self.gpu.update_entry(queue, region, tile_x, tile_y, slot);
    }
}
