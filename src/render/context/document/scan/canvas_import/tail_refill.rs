use crate::core::scanner::FileItem;
use crate::render::layout::{
    append_files_to_block_tail_at_anchor, populated_block_extent, relayout_block_at_anchor,
    BlockGridAddress, LayoutBlockPlan, SceneLayoutCursor, TailAppendPlan, BLOCK_FILE_CAP,
};

#[derive(Clone)]
pub(in crate::render::context::document::scan) struct TailRefillSeed {
    pub(in crate::render::context::document::scan) block_id: u64,
    pub(in crate::render::context::document::scan) grid: BlockGridAddress,
    pub(in crate::render::context::document::scan) existing_len: usize,
    pub(in crate::render::context::document::scan) bounds: [f32; 4],
    pub(in crate::render::context::document::scan) files: Vec<FileItem>,
}

#[derive(Clone)]
pub(in crate::render::context::document::scan) struct TailAppendSeed {
    pub(in crate::render::context::document::scan) block_id: u64,
    pub(in crate::render::context::document::scan) grid: BlockGridAddress,
    pub(in crate::render::context::document::scan) existing_len: usize,
    pub(in crate::render::context::document::scan) bounds: [f32; 4],
}

pub(super) struct TailRefillState {
    seed: TailRefillSeed,
    combined_files: Vec<FileItem>,
    new_consumed: usize,
    emitted: bool,
    plan: Option<LayoutBlockPlan>,
}

pub(super) struct TailAppendState {
    seed: TailAppendSeed,
    emitted_new: usize,
}

impl TailRefillState {
    pub(super) fn new(seed: TailRefillSeed) -> Self {
        let keep = seed.existing_len.min(seed.files.len());
        Self {
            combined_files: seed.files[..keep].to_vec(),
            seed,
            new_consumed: 0,
            emitted: false,
            plan: None,
        }
    }

    pub(super) fn needs_more(&self) -> bool {
        self.combined_files.len() < BLOCK_FILE_CAP
    }

    pub(super) fn consume_new_file(&mut self, file: FileItem) {
        if !self.needs_more() {
            return;
        }
        self.combined_files.push(file);
        self.new_consumed = self.new_consumed.saturating_add(1);
        if !self.needs_more() {
            self.finalize_plan();
        }
    }

    pub(super) fn finalize_at_end(&mut self) {
        if self.new_consumed == 0 {
            return;
        }
        self.finalize_plan();
    }

    pub(super) fn take_plan_once(&mut self) -> Option<LayoutBlockPlan> {
        if self.emitted {
            return None;
        }
        let plan = self.plan.take();
        if plan.is_some() {
            self.emitted = true;
        }
        plan
    }

    pub(super) fn combined_len(&self) -> usize {
        self.combined_files.len()
    }

    pub(super) fn seed_bounds(&self) -> [f32; 4] {
        self.seed.bounds
    }

    pub(super) fn refill_bounds(&self) -> [f32; 4] {
        self.plan
            .as_ref()
            .map(|plan| plan.bounds)
            .unwrap_or(self.seed.bounds)
    }

    fn finalize_plan(&mut self) {
        if self.plan.is_some() || self.new_consumed == 0 {
            return;
        }
        self.plan = relayout_block_at_anchor(
            self.combined_files.as_slice(),
            self.seed.block_id,
            self.seed.grid,
            self.seed.bounds[0],
            self.seed.bounds[3],
        );
    }
}

impl TailAppendState {
    pub(super) fn new(seed: TailAppendSeed) -> Self {
        Self {
            seed,
            emitted_new: 0,
        }
    }

    pub(super) fn needs_more(&self) -> bool {
        self.seed.existing_len.saturating_add(self.emitted_new) < BLOCK_FILE_CAP
    }

    pub(super) fn remaining_capacity(&self) -> usize {
        BLOCK_FILE_CAP
            .saturating_sub(self.seed.existing_len)
            .saturating_sub(self.emitted_new)
    }

    pub(super) fn current_bounds(&self) -> [f32; 4] {
        tail_append_bounds(self.seed.bounds, self.seed.existing_len, self.emitted_new)
    }

    pub(super) fn consume_chunk(&mut self, files: &[FileItem]) -> Option<TailAppendPlan> {
        let take = self.remaining_capacity().min(files.len());
        if take == 0 {
            return None;
        }

        let start_local_idx = self.seed.existing_len.saturating_add(self.emitted_new);
        let plan = append_files_to_block_tail_at_anchor(
            &files[..take],
            self.seed.block_id,
            self.seed.grid,
            self.seed.bounds[0],
            self.seed.bounds[3],
            start_local_idx,
        )?;
        self.emitted_new = self.emitted_new.saturating_add(plan.entries.len());
        Some(plan)
    }
}

pub(super) fn fixed_grid_cursor_for_append(start_cursor: SceneLayoutCursor) -> SceneLayoutCursor {
    let mut cursor = start_cursor;
    cursor.normalize_block_grid();
    cursor
}

fn tail_append_bounds(bounds: [f32; 4], existing_len: usize, appended_len: usize) -> [f32; 4] {
    let new_len = existing_len.saturating_add(appended_len);
    if appended_len == 0 || new_len == 0 {
        return bounds;
    }
    let (width, height) = populated_block_extent(new_len);
    [bounds[0], bounds[3] - height, bounds[0] + width, bounds[3]]
}

pub(super) fn merge_bounds_opt(base: Option<[f32; 4]>, extra: [f32; 4]) -> Option<[f32; 4]> {
    if let Some(bounds) = base {
        return Some([
            bounds[0].min(extra[0]),
            bounds[1].min(extra[1]),
            bounds[2].max(extra[2]),
            bounds[3].max(extra[3]),
        ]);
    }
    Some(extra)
}

#[cfg(test)]
mod tests {
    use super::fixed_grid_cursor_for_append;
    use crate::render::layout::{block_grid_span, SceneLayoutCursor};

    #[test]
    fn append_cursor_always_uses_fixed_block_grid() {
        let cursor = SceneLayoutCursor::new_centered(8.0);
        let fixed = fixed_grid_cursor_for_append(cursor);

        assert!((fixed.grid_cell_w - block_grid_span()).abs() < 0.001);
        assert!((fixed.grid_cell_h - block_grid_span()).abs() < 0.001);
    }

    #[test]
    fn append_cursor_normalizes_existing_grid_dimensions() {
        let mut cursor = SceneLayoutCursor::new_centered(8.0);
        cursor.grid_cell_w = 123.0;
        cursor.grid_cell_h = 456.0;

        let fixed = fixed_grid_cursor_for_append(cursor);

        assert!((fixed.grid_cell_w - block_grid_span()).abs() < 0.001);
        assert!((fixed.grid_cell_h - block_grid_span()).abs() < 0.001);
    }
}
