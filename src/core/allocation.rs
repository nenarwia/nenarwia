pub struct PagedAllocator {
    free_slots: Vec<u32>,
}

impl PagedAllocator {
    pub fn new(max_slots: u32) -> Self {
        let free_slots = (0..max_slots).rev().collect();
        Self { free_slots }
    }

    pub fn allocate(&mut self) -> Option<u32> {
        self.free_slots.pop()
    }

    pub fn free(&mut self, slot: u32) {
        self.free_slots.push(slot);
    }
}
