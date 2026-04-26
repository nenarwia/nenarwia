use std::collections::{HashMap, VecDeque};

#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub struct TileId {
    pub asset_key: u64,
    pub lod: u8,
    pub x: u32,
    pub y: u32,
}

pub type CanvasMediaSlotId = TileId;

pub struct PageTable {
    pub mapping: HashMap<TileId, u32>,
    pub free_slots: Vec<u32>,
    pub lru_queue: VecDeque<(TileId, u64)>,
    pub last_used: HashMap<TileId, u64>,
    usage_counter: u64,
    pub slot_to_tile: Vec<Option<TileId>>,
}

impl PageTable {
    pub fn new(total_slots: u32) -> Self {
        let free_slots = (0..total_slots).rev().collect();
        let slot_to_tile = vec![None; total_slots as usize];

        Self {
            mapping: HashMap::new(),
            free_slots,
            lru_queue: VecDeque::new(),
            last_used: HashMap::new(),
            usage_counter: 0,
            slot_to_tile,
        }
    }

    pub fn get_slot(&self, tile: TileId) -> Option<u32> {
        self.mapping.get(&tile).copied()
    }
    pub fn touch(&mut self, tile: TileId) {
        if self.mapping.contains_key(&tile) {
            self.usage_counter = self.usage_counter.wrapping_add(1);
            let stamp = self.usage_counter;
            self.last_used.insert(tile, stamp);
            self.lru_queue.push_back((tile, stamp));
        }
    }
    pub fn invalidate_asset_lod(&mut self, asset_key: u64, lod: u8) -> usize {
        let mut victims: Vec<(TileId, u32)> = Vec::with_capacity(256);

        for (tile, slot) in self.mapping.iter() {
            if tile.asset_key == asset_key && tile.lod == lod {
                victims.push((*tile, *slot));
            }
        }

        for (tile, slot) in victims.iter() {
            self.mapping.remove(tile);
            self.last_used.remove(tile);
            if (*slot as usize) < self.slot_to_tile.len() {
                self.slot_to_tile[*slot as usize] = None;
            }
            self.free_slots.push(*slot);
        }

        victims.len()
    }
    pub fn allocate(&mut self, new_tile: TileId) -> Option<(u32, Option<TileId>)> {
        self.usage_counter = self.usage_counter.wrapping_add(1);
        let stamp = self.usage_counter;
        self.last_used.insert(new_tile, stamp);
        if let Some(slot) = self.free_slots.pop() {
            if (slot as usize) >= self.slot_to_tile.len() {
                self.last_used.remove(&new_tile);
                return None;
            }
            self.mapping.insert(new_tile, slot);
            self.slot_to_tile[slot as usize] = Some(new_tile);
            self.lru_queue.push_back((new_tile, stamp));
            return Some((slot, None));
        }
        while let Some((victim_tile, victim_stamp)) = self.lru_queue.pop_front() {
            if self.last_used.get(&victim_tile).copied() != Some(victim_stamp) {
                continue;
            }
            if let Some(slot) = self.mapping.remove(&victim_tile) {
                let slot_idx = slot as usize;
                if slot_idx >= self.slot_to_tile.len() {
                    self.last_used.remove(&victim_tile);
                    continue;
                }
                self.mapping.insert(new_tile, slot);
                self.slot_to_tile[slot_idx] = Some(new_tile);
                self.lru_queue.push_back((new_tile, stamp));
                self.last_used.remove(&victim_tile);

                return Some((slot, Some(victim_tile)));
            }
        }
        if let Some((&victim_tile, &slot)) = self.mapping.iter().next() {
            let slot_idx = slot as usize;
            if slot_idx < self.slot_to_tile.len() {
                self.mapping.remove(&victim_tile);
                self.last_used.remove(&victim_tile);
                self.mapping.insert(new_tile, slot);
                self.slot_to_tile[slot_idx] = Some(new_tile);
                self.lru_queue.push_back((new_tile, stamp));
                return Some((slot, Some(victim_tile)));
            }
        }

        self.last_used.remove(&new_tile);
        None
    }
}
