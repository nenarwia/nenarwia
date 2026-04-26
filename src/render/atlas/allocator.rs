use crate::core::allocation::PagedAllocator;

pub struct AtlasAllocator {
    pub allocator: PagedAllocator,
    pub slot_last_used: Vec<u64>,
    pub slot_owner_id: Vec<Option<u64>>,
}

impl AtlasAllocator {
    pub fn new(total_slots: u32) -> Self {
        Self {
            allocator: PagedAllocator::new(total_slots),
            slot_last_used: vec![0; total_slots as usize],
            slot_owner_id: vec![None; total_slots as usize],
        }
    }

    pub fn touch(&mut self, slot: u32, frame: u64) {
        if (slot as usize) < self.slot_last_used.len() {
            self.slot_last_used[slot as usize] = frame;
        }
    }

    pub fn allocate_with_policy<F>(
        &mut self,
        current_frame: u64,
        mut eviction_rank: F,
    ) -> (Option<u32>, Option<u64>)
    where
        F: FnMut(Option<u64>) -> Option<u8>,
    {
        if let Some(slot) = self.allocator.allocate() {
            return (Some(slot), None);
        }

        let mut best_rank = u8::MAX;
        let mut oldest = u64::MAX;
        let mut victim = None;
        for (i, &last) in self.slot_last_used.iter().enumerate() {
            if last >= current_frame {
                continue;
            }
            let owner = self.slot_owner_id[i];
            let Some(rank) = eviction_rank(owner) else {
                continue;
            };
            if rank < best_rank || (rank == best_rank && last < oldest) {
                best_rank = rank;
                oldest = last;
                victim = Some(i as u32);
            }
        }

        if let Some(slot) = victim {
            let evicted = self.slot_owner_id[slot as usize];
            (Some(slot), evicted)
        } else {
            (None, None)
        }
    }

    pub fn mark_used(&mut self, slot: u32, id: u64, frame: u64) {
        if (slot as usize) < self.slot_last_used.len() {
            self.slot_last_used[slot as usize] = frame;
            self.slot_owner_id[slot as usize] = Some(id);
        }
    }

    pub fn clear_slot(&mut self, slot: u32) {
        if (slot as usize) < self.slot_last_used.len() {
            self.slot_last_used[slot as usize] = 0;
            self.slot_owner_id[slot as usize] = None;
            self.allocator.free(slot);
        }
    }
}
