use super::super::types::{
    BlockGridAddress, LayoutBlockBatchPlan, LayoutBlockPlan, SceneLayoutCursor, BLOCK_FILE_CAP,
};
use super::placement::block_slot_for_index;
use super::plans::{build_anchored_block_plan, estimated_layout_extent};
use crate::core::scanner::FileItem;

pub struct BlockLayoutPlanner {
    cursor: SceneLayoutCursor,
    next_block_id: u64,
    pending_files: Vec<FileItem>,
    planned_blocks: Vec<LayoutBlockPlan>,
    layout_bounds: Option<[f32; 4]>,
}

impl BlockLayoutPlanner {
    pub fn new(cursor: SceneLayoutCursor, next_block_id: u64) -> Self {
        Self::new_with_seed_bounds(cursor, next_block_id, None)
    }

    pub fn new_with_seed_bounds(
        mut cursor: SceneLayoutCursor,
        next_block_id: u64,
        seed_bounds: Option<[f32; 4]>,
    ) -> Self {
        cursor.normalize_block_grid();
        Self {
            cursor,
            next_block_id,
            pending_files: Vec::new(),
            planned_blocks: Vec::new(),
            layout_bounds: seed_bounds,
        }
    }

    pub fn grow_target_for_total_items(&mut self, total_items: usize) {
        self.cursor
            .grow_target_side(estimated_layout_extent(total_items).max(8.0));
    }

    pub fn push(&mut self, file: FileItem) {
        self.pending_files.push(file);
        while self.pending_files.len() >= BLOCK_FILE_CAP {
            self.flush_block(BLOCK_FILE_CAP);
        }
    }

    pub fn push_many(&mut self, files: &[FileItem]) {
        for file in files.iter().cloned() {
            self.push(file);
        }
    }

    pub fn flush_pending_block(&mut self) {
        if self.pending_files.is_empty() {
            return;
        }

        let count = self.pending_files.len();
        self.flush_block(count);
    }

    pub fn take_planned_blocks(&mut self) -> Vec<LayoutBlockPlan> {
        std::mem::take(&mut self.planned_blocks)
    }

    pub fn cursor(&self) -> SceneLayoutCursor {
        self.cursor
    }

    pub fn take_batch(&mut self) -> LayoutBlockBatchPlan {
        let (layout_width, layout_height) = self.current_layout_size();
        LayoutBlockBatchPlan {
            blocks: self.take_planned_blocks(),
            cursor: self.cursor,
            layout_width,
            layout_height,
        }
    }

    pub fn finish(mut self) -> LayoutBlockBatchPlan {
        self.flush_pending_block();
        let (layout_width, layout_height) = self.current_layout_size();
        LayoutBlockBatchPlan {
            blocks: self.planned_blocks,
            cursor: self.cursor,
            layout_width,
            layout_height,
        }
    }

    fn current_layout_size(&self) -> (f32, f32) {
        let Some(bounds) = self.layout_bounds else {
            let side = self.cursor.target_side.max(8.0);
            return (side, side);
        };

        let width = (bounds[2] - bounds[0])
            .max(0.0)
            .max(self.cursor.target_side);
        let height = (bounds[3] - bounds[1])
            .max(0.0)
            .max(self.cursor.target_side);
        (width, height)
    }

    fn flush_block(&mut self, count: usize) {
        if count == 0 {
            return;
        }

        let files: Vec<FileItem> = self.pending_files.drain(..count).collect();
        let (grid_x, grid_y) = block_slot_for_index(self.next_block_id);
        let grid = BlockGridAddress {
            col: grid_x as i32,
            row: grid_y as i32,
        };
        let anchor_left = self.cursor.left_edge + grid_x as f32 * self.cursor.grid_cell_w;
        let anchor_top = self.cursor.top_edge - grid_y as f32 * self.cursor.grid_cell_h;

        let Some(plan) = build_anchored_block_plan(
            files.as_slice(),
            self.next_block_id,
            grid,
            anchor_left,
            anchor_top,
        ) else {
            return;
        };

        self.absorb_bounds(plan.bounds);
        self.planned_blocks.push(plan);
        self.next_block_id = self.next_block_id.saturating_add(1);
    }

    fn absorb_bounds(&mut self, bounds: [f32; 4]) {
        self.layout_bounds = Some(match self.layout_bounds {
            Some(existing) => [
                existing[0].min(bounds[0]),
                existing[1].min(bounds[1]),
                existing[2].max(bounds[2]),
                existing[3].max(bounds[3]),
            ],
            None => bounds,
        });
    }
}
