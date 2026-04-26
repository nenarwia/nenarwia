use serde::{Deserialize, Serialize};

use crate::core::scanner::FileItem;
use crate::render::instance::InstanceRaw;

pub const BLOCK_FILE_CAP: usize = 256;
pub const BLOCK_SLOT_COLUMNS: usize = 16;
pub const SLOT_SIDE: f32 = 1.5;
pub const SLOT_GAP: f32 = 0.05;
pub const BLOCK_GAP: f32 = 0.75;

pub fn full_block_content_span() -> f32 {
    SLOT_SIDE * BLOCK_SLOT_COLUMNS as f32 + SLOT_GAP * (BLOCK_SLOT_COLUMNS.saturating_sub(1)) as f32
}

pub fn block_grid_span() -> f32 {
    full_block_content_span() + BLOCK_GAP
}

pub fn block_slot_columns_for_count(count: usize) -> usize {
    if count == 0 {
        return 0;
    }

    BLOCK_SLOT_COLUMNS
}

pub fn block_slot_rows_for_count(count: usize) -> usize {
    let cols = block_slot_columns_for_count(count);
    if cols == 0 {
        0
    } else {
        count.div_ceil(cols)
    }
}

pub fn slot_grid_position(local_idx: usize, slot_cols: usize) -> (usize, usize) {
    if slot_cols == 0 {
        return (0, 0);
    }

    (local_idx % slot_cols, local_idx / slot_cols)
}

pub fn slot_grid_local_index(slot_col: usize, slot_row: usize, slot_cols: usize) -> usize {
    if slot_cols == 0 {
        return 0;
    }

    slot_row.saturating_mul(slot_cols).saturating_add(slot_col)
}

pub fn populated_block_extent(count: usize) -> (f32, f32) {
    if count == 0 {
        return (0.0, 0.0);
    }

    let cols = count.min(BLOCK_SLOT_COLUMNS);
    let rows = count.div_ceil(BLOCK_SLOT_COLUMNS);
    let width = SLOT_SIDE * cols as f32 + SLOT_GAP * cols.saturating_sub(1) as f32;
    let height = SLOT_SIDE * rows as f32 + SLOT_GAP * rows.saturating_sub(1) as f32;
    (width, height)
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneLayoutCursor {
    pub target_side: f32,
    pub left_edge: f32,
    pub top_edge: f32,
    pub grid_cell_w: f32,
    pub grid_cell_h: f32,
}

impl SceneLayoutCursor {
    pub fn new_centered(target_side: f32) -> Self {
        let target_side = target_side.max(8.0);
        let left_edge = -target_side * 0.5;
        let top_edge = target_side * 0.5;
        Self {
            target_side,
            left_edge,
            top_edge,
            grid_cell_w: block_grid_span(),
            grid_cell_h: block_grid_span(),
        }
    }

    pub fn grow_target_side(&mut self, side: f32) {
        self.target_side = self.target_side.max(side.max(8.0));
    }

    pub fn normalize_block_grid(&mut self) {
        let span = block_grid_span();
        self.grid_cell_w = span;
        self.grid_cell_h = span;
    }
}

impl Default for SceneLayoutCursor {
    fn default() -> Self {
        Self::new_centered(8.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlockGridAddress {
    pub col: i32,
    pub row: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SlotGridAddress {
    pub block: BlockGridAddress,
    pub col: u8,
    pub row: u8,
}

#[derive(Clone, Debug)]
pub struct LayoutBlockEntry {
    pub raw: InstanceRaw,
    pub file: FileItem,
}

#[derive(Clone, Debug)]
pub struct LayoutBlockPlan {
    pub block_id: u64,
    pub grid: BlockGridAddress,
    pub bounds: [f32; 4],
    pub entries: Vec<LayoutBlockEntry>,
}

#[derive(Clone, Debug)]
pub struct TailAppendPlan {
    pub block_id: u64,
    pub grid: BlockGridAddress,
    pub start_local_idx: usize,
    pub bounds: [f32; 4],
    pub entries: Vec<LayoutBlockEntry>,
}

#[derive(Clone, Debug)]
pub struct LayoutBlockBatchPlan {
    pub blocks: Vec<LayoutBlockPlan>,
    pub cursor: SceneLayoutCursor,
    pub layout_width: f32,
    pub layout_height: f32,
}

impl LayoutBlockBatchPlan {
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
}
