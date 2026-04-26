use super::allocator::AtlasAllocator;
use super::gpu::AtlasGpu;

pub struct TextureAtlas {
    pub gpu: AtlasGpu,
    lru: AtlasAllocator,
    pub atlas_size: u32,
    pub page_size: u32,
    pub columns: u32,
}

impl TextureAtlas {
    /// Tiered atlas constructor.
    pub fn new_with_page_size(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        requested_size: u32,
        max_dim: u32,
        page_size: u32,
    ) -> Self {
        let page_size = page_size.max(1);

        let mut atlas_size = requested_size.max(page_size).min(max_dim);

        // Snap to grid
        atlas_size = (atlas_size / page_size) * page_size;
        atlas_size = atlas_size.max(page_size);

        let columns = atlas_size / page_size;

        let gpu = AtlasGpu::new(device, atlas_size);
        let lru = AtlasAllocator::new(columns * columns);

        log::info!(
            "Atlas: {}x{} ({} slots)",
            atlas_size,
            atlas_size,
            columns * columns
        );

        Self {
            gpu,
            lru,
            atlas_size,
            page_size,
            columns,
        }
    }

    pub fn touch_slot(&mut self, slot: u32, frame: u64) {
        self.lru.touch(slot, frame);
    }

    pub fn allocate_lru_with_policy<F>(
        &mut self,
        frame: u64,
        eviction_rank: F,
    ) -> (Option<u32>, Option<u64>)
    where
        F: FnMut(Option<u64>) -> Option<u8>,
    {
        self.lru.allocate_with_policy(frame, eviction_rank)
    }

    pub fn upload_to_slot(
        &mut self,
        queue: &wgpu::Queue,
        slot: u32,
        id: u64,
        data: &[u8],
        frame: u64,
    ) -> [f32; 4] {
        self.lru.mark_used(slot, id, frame);
        let (col, row) = (slot % self.columns, slot / self.columns);
        let (x, y) = (col * self.page_size, row * self.page_size);
        self.gpu
            .upload_region(queue, x, y, self.page_size, self.page_size, data);
        let s = self.atlas_size as f32;
        [
            x as f32 / s,
            y as f32 / s,
            self.page_size as f32 / s,
            self.page_size as f32 / s,
        ]
    }

    pub fn uv_for_slot(&self, slot: u32) -> [f32; 4] {
        let (col, row) = (slot % self.columns, slot / self.columns);
        let (x, y) = (col * self.page_size, row * self.page_size);
        let s = self.atlas_size as f32;
        [
            x as f32 / s,
            y as f32 / s,
            self.page_size as f32 / s,
            self.page_size as f32 / s,
        ]
    }

    pub fn remove_id(&mut self, queue: &wgpu::Queue, id: u64) -> bool {
        let Some(slot) = self
            .lru
            .slot_owner_id
            .iter()
            .position(|owner| *owner == Some(id))
            .map(|slot| slot as u32)
        else {
            return false;
        };

        let zero_data = vec![0u8; (self.page_size * self.page_size * 4) as usize];
        let (col, row) = (slot % self.columns, slot / self.columns);
        let (x, y) = (col * self.page_size, row * self.page_size);
        self.gpu
            .upload_region(queue, x, y, self.page_size, self.page_size, &zero_data);
        self.lru.clear_slot(slot);
        true
    }

    pub fn clear(&mut self) {
        let total_slots = self.lru.slot_last_used.len() as u32;
        self.lru = AtlasAllocator::new(total_slots);
    }
}
