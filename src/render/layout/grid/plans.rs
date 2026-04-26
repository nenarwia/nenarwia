use super::super::types::{
    block_grid_span, block_slot_columns_for_count, full_block_content_span, populated_block_extent,
    BlockGridAddress, LayoutBlockEntry, LayoutBlockPlan, TailAppendPlan, BLOCK_FILE_CAP, SLOT_SIDE,
};
use super::placement::{
    anchored_slot_center, build_item_instance, corner_square_grid_items, raw_items_bounds,
};
use crate::core::scanner::FileItem;
use crate::render::instance::InstanceRaw;

pub fn estimated_layout_extent(total_items: usize) -> f32 {
    if total_items == 0 {
        return 8.0;
    }

    let block_count = total_items.div_ceil(BLOCK_FILE_CAP).max(1);
    let grid_side = (block_count as f32).sqrt().ceil().max(1.0);
    let side = grid_side * block_grid_span() - (block_grid_span() - full_block_content_span());
    side.clamp(8.0, 4096.0)
}

pub fn relayout_block_at_anchor(
    files: &[FileItem],
    block_id: u64,
    grid: BlockGridAddress,
    left: f32,
    top: f32,
) -> Option<LayoutBlockPlan> {
    build_anchored_block_plan(files, block_id, grid, left, top)
}

pub fn append_files_to_block_tail_at_anchor(
    files: &[FileItem],
    block_id: u64,
    grid: BlockGridAddress,
    left: f32,
    top: f32,
    start_local_idx: usize,
) -> Option<TailAppendPlan> {
    if files.is_empty() || start_local_idx >= BLOCK_FILE_CAP {
        return None;
    }

    let capped_count = files
        .len()
        .min(BLOCK_FILE_CAP.saturating_sub(start_local_idx));
    if capped_count == 0 {
        return None;
    }

    let mut entries = Vec::with_capacity(capped_count);
    let new_len = start_local_idx.saturating_add(capped_count);
    let slot_cols = block_slot_columns_for_count(new_len);
    for (offset, file) in files.iter().take(capped_count).cloned().enumerate() {
        let local_idx = start_local_idx.saturating_add(offset);
        let (x, y) = anchored_slot_center(left, top, local_idx, slot_cols);
        entries.push(LayoutBlockEntry {
            raw: build_item_instance(&file, x, y, SLOT_SIDE, SLOT_SIDE),
            file,
        });
    }
    if entries.is_empty() {
        return None;
    }

    let (width, height) = populated_block_extent(new_len);
    Some(TailAppendPlan {
        block_id,
        grid,
        start_local_idx,
        bounds: [left, top - height, left + width, top],
        entries,
    })
}

pub(super) fn build_anchored_block_plan(
    files: &[FileItem],
    block_id: u64,
    grid: BlockGridAddress,
    left: f32,
    top: f32,
) -> Option<LayoutBlockPlan> {
    let items = corner_square_grid_items(files);
    build_anchored_block_plan_from_items(files, items, block_id, grid, left, top)
}

fn build_anchored_block_plan_from_items(
    files: &[FileItem],
    items: Vec<InstanceRaw>,
    block_id: u64,
    grid: BlockGridAddress,
    left: f32,
    top: f32,
) -> Option<LayoutBlockPlan> {
    if items.is_empty() {
        return None;
    }

    let local_bounds = raw_items_bounds(items.as_slice())?;
    let offset_x = left - local_bounds[0];
    let offset_y = top - local_bounds[3];

    let mut entries = Vec::with_capacity(items.len());
    for (local_idx, mut raw) in items.into_iter().enumerate() {
        let file = files.get(local_idx)?.clone();
        raw.data[0] += offset_x;
        raw.data[1] += offset_y;
        entries.push(LayoutBlockEntry { raw, file });
    }
    if entries.is_empty() {
        return None;
    }

    Some(LayoutBlockPlan {
        block_id,
        grid,
        bounds: [
            local_bounds[0] + offset_x,
            local_bounds[1] + offset_y,
            local_bounds[2] + offset_x,
            local_bounds[3] + offset_y,
        ],
        entries,
    })
}
