use crate::render::instance::InstanceRaw;
use crate::render::layout::{
    BlockGridAddress, SceneLayoutCursor, SlotGridAddress, SLOT_GAP, SLOT_SIDE,
};
use std::collections::HashMap;

pub struct SceneLayoutBlock {
    pub block_id: u64,
    pub grid: BlockGridAddress,
    pub bounds: [f32; 4],
    pub index_start: usize,
    pub index_len: usize,
}

pub struct Scene {
    pub layout_blocks: Vec<SceneLayoutBlock>,
    pub layout_cursor: SceneLayoutCursor,
    pub all_items_raw: Vec<InstanceRaw>,
    pub slot_addresses: Vec<SlotGridAddress>,
    pub total_count: usize,
    pub layout_width: f32,
    pub layout_height: f32,
    pub block_grid_lookup: HashMap<BlockGridAddress, usize>,
    pub id_to_index: HashMap<u64, usize>,
    pub index_to_id: Vec<u64>,
    pub asset_keys: Vec<u64>,
    pub asset_key_to_index: HashMap<u64, usize>,
    pub quality_debt: Vec<f32>,
    pub item_dimensions: Vec<(u32, u32)>,
    pub last_lod: Vec<u8>,
    pub display_lod: Vec<u8>,
    pub render_lod: Vec<u8>,
    pub coarse_lod: Vec<u8>,
}

impl Scene {
    pub fn next_block_id(&self) -> u64 {
        self.layout_blocks
            .last()
            .map(|block| block.block_id.saturating_add(1))
            .unwrap_or(0)
    }

    pub fn layout_bounds(&self) -> Option<[f32; 4]> {
        if self.layout_blocks.is_empty() {
            return None;
        }

        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for block in self.layout_blocks.iter() {
            min_x = min_x.min(block.bounds[0]);
            min_y = min_y.min(block.bounds[1]);
            max_x = max_x.max(block.bounds[2]);
            max_y = max_y.max(block.bounds[3]);
        }
        if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
            return None;
        }
        Some([min_x, min_y, max_x, max_y])
    }

    pub fn content_bounds(&self) -> Option<(f32, f32, f32, f32)> {
        self.layout_bounds()
            .map(|[min_x, min_y, max_x, max_y]| (min_x, min_y, max_x, max_y))
    }

    pub fn refresh_layout_extent_from_blocks(&mut self) {
        if let Some(bounds) = self.layout_bounds() {
            let width = (bounds[2] - bounds[0]).max(0.0);
            let height = (bounds[3] - bounds[1]).max(0.0);
            self.layout_width = width.max(self.layout_cursor.target_side);
            self.layout_height = height.max(self.layout_cursor.target_side);
            return;
        }

        if self.total_count == 0 {
            self.layout_width = 0.0;
            self.layout_height = 0.0;
        } else {
            self.layout_width = self.layout_cursor.target_side.max(8.0);
            self.layout_height = self.layout_cursor.target_side.max(8.0);
        }
    }

    pub fn index_for_id(&self, id: u64) -> Option<usize> {
        self.id_to_index.get(&id).copied()
    }

    pub fn index_for_asset(&self, asset_key: u64) -> Option<usize> {
        self.asset_key_to_index.get(&asset_key).copied()
    }

    pub fn block_for_grid(&self, grid: BlockGridAddress) -> Option<&SceneLayoutBlock> {
        self.block_grid_lookup
            .get(&grid)
            .and_then(|&idx| self.layout_blocks.get(idx))
    }

    pub fn item_slot_address(&self, idx: usize) -> Option<SlotGridAddress> {
        self.slot_addresses.get(idx).copied()
    }

    pub fn item_world_center(&self, idx: usize) -> Option<(f64, f64)> {
        let address = self.item_slot_address(idx)?;
        let (block_left, block_top) = self.block_origin_world(address.block);
        let step = slot_step_world();
        let x = block_left + address.col as f64 * step + SLOT_SIDE as f64 * 0.5;
        let y = block_top - address.row as f64 * step - SLOT_SIDE as f64 * 0.5;
        Some((x, y))
    }

    pub fn item_draw_raw(&self, idx: usize, render_origin: (f64, f64)) -> Option<InstanceRaw> {
        let mut raw = *self.all_items_raw.get(idx)?;
        let (x, y) = self.item_world_center(idx)?;
        raw.data[0] = (x - render_origin.0) as f32;
        raw.data[1] = (y - render_origin.1) as f32;
        Some(raw)
    }

    pub fn item_fitted_world_geometry(&self, idx: usize) -> Option<(f64, f64, f32, f32)> {
        let raw = self.all_items_raw.get(idx)?;
        let (slot_x, slot_y) = self.item_world_center(idx)?;
        let slot_w = raw.data[2].max(0.0);
        let slot_h = raw.data[3].max(0.0);
        let slot_left = slot_x - slot_w as f64 * 0.5;
        let slot_top = slot_y + slot_h as f64 * 0.5;
        let fit_x = raw.fit_rect[0].clamp(0.0, 1.0);
        let fit_y = raw.fit_rect[1].clamp(0.0, 1.0);
        let fit_w = raw.fit_rect[2].clamp(0.0, 1.0);
        let fit_h = raw.fit_rect[3].clamp(0.0, 1.0);
        let media_w = slot_w * fit_w;
        let media_h = slot_h * fit_h;
        let media_left = slot_left + slot_w as f64 * fit_x as f64;
        let media_top = slot_top - slot_h as f64 * fit_y as f64;
        Some((
            media_left + media_w as f64 * 0.5,
            media_top - media_h as f64 * 0.5,
            media_w,
            media_h,
        ))
    }

    pub fn bounds(&self) -> Option<(f32, f32, f32, f32)> {
        if self.total_count == 0 {
            return None;
        }
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for idx in 0..self.total_count {
            let Some(raw) = self.all_items_raw.get(idx) else {
                continue;
            };
            let w = raw.data[2].max(0.0);
            let h = raw.data[3].max(0.0);
            if w <= 0.0 || h <= 0.0 {
                continue;
            }
            let Some((cx, cy)) = self.item_world_center(idx) else {
                continue;
            };
            let cx = cx as f32;
            let cy = cy as f32;
            let half_w = w * 0.5;
            let half_h = h * 0.5;
            let left = cx - half_w;
            let right = cx + half_w;
            let top = cy + half_h;
            let bottom = cy - half_h;

            min_x = min_x.min(left);
            max_x = max_x.max(right);
            min_y = min_y.min(bottom);
            max_y = max_y.max(top);
        }

        if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
            return None;
        }
        Some((min_x, min_y, max_x, max_y))
    }

    pub fn block_origin_world(&self, grid: BlockGridAddress) -> (f64, f64) {
        (
            self.layout_cursor.left_edge as f64
                + grid.col as f64 * self.layout_cursor.grid_cell_w as f64,
            self.layout_cursor.top_edge as f64
                - grid.row as f64 * self.layout_cursor.grid_cell_h as f64,
        )
    }
}

fn slot_step_world() -> f64 {
    (SLOT_SIDE + SLOT_GAP) as f64
}
