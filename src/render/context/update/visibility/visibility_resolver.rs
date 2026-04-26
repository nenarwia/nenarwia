use crate::render::context::state::{RenderContext, VisibleItem};
use crate::render::layout::{
    block_slot_columns_for_count, block_slot_rows_for_count, slot_grid_local_index,
    BlockGridAddress, SLOT_GAP, SLOT_SIDE,
};

pub(super) struct VisibleItemsProbe {
    pub items: Vec<VisibleItem>,
    pub reached_limit: bool,
}

pub(super) fn probe_visible_items_with_limit(
    ctx: &RenderContext,
    limit: usize,
) -> VisibleItemsProbe {
    let (view_min_x, view_max_x, view_min_y, view_max_y) = ctx.view().viewport_rect();
    let mut visible_items = Vec::new();
    let mut reached_limit = false;

    if let Some((min_col, max_col, min_row, max_row)) = visible_block_grid_range(ctx) {
        for block_row in min_row..=max_row {
            for block_col in min_col..=max_col {
                let Some(block) = ctx.scene.block_for_grid(BlockGridAddress {
                    col: block_col,
                    row: block_row,
                }) else {
                    continue;
                };
                let (block_left, block_top) = ctx.scene.block_origin_world(block.grid);
                let slot_cols = block_slot_columns_for_count(block.index_len);
                let slot_rows = block_slot_rows_for_count(block.index_len);
                if slot_cols == 0 || slot_rows == 0 {
                    continue;
                }
                let (min_slot_col, max_slot_col, min_slot_row, max_slot_row) =
                    visible_slot_range_in_block(
                        view_min_x, view_max_x, view_min_y, view_max_y, block_left, block_top,
                        slot_cols, slot_rows,
                    );
                for slot_row in min_slot_row..=max_slot_row {
                    for slot_col in min_slot_col..=max_slot_col {
                        let local_idx = slot_grid_local_index(slot_col, slot_row, slot_cols);
                        if local_idx >= block.index_len {
                            continue;
                        }
                        let (slot_left, slot_bottom, slot_right, slot_top) =
                            slot_world_bounds(block_left, block_top, slot_col, slot_row);
                        if !rects_intersect(
                            view_min_x,
                            view_min_y,
                            view_max_x,
                            view_max_y,
                            slot_left,
                            slot_bottom,
                            slot_right,
                            slot_top,
                        ) {
                            continue;
                        }

                        let idx = block.index_start.saturating_add(local_idx);
                        let Some(id) = ctx.scene.index_to_id.get(idx).copied() else {
                            continue;
                        };
                        visible_items.push(VisibleItem { id, idx });
                        if visible_items.len() >= limit {
                            reached_limit = true;
                            break;
                        }
                    }
                    if reached_limit {
                        break;
                    }
                }
                if reached_limit {
                    break;
                }
            }
            if reached_limit {
                break;
            }
        }
    }

    VisibleItemsProbe {
        items: visible_items,
        reached_limit,
    }
}

pub(super) fn collect_manifested_media_items(ctx: &RenderContext) -> Vec<VisibleItem> {
    let mut visible_items = Vec::new();

    for (idx, raw) in ctx.scene.all_items_raw.iter().copied().enumerate() {
        if !super::slot_has_media_content(raw) {
            continue;
        }
        let Some(id) = ctx.scene.index_to_id.get(idx).copied() else {
            continue;
        };
        visible_items.push(VisibleItem { id, idx });
    }

    visible_items
}

pub(crate) fn item_id_at_world_point(ctx: &RenderContext, point: [f64; 2]) -> Option<u64> {
    let grid = block_grid_for_point(ctx, point)?;
    let block = ctx.scene.block_for_grid(grid)?;
    let (block_left, block_top) = ctx.scene.block_origin_world(block.grid);
    let slot_cols = block_slot_columns_for_count(block.index_len);
    let slot_rows = block_slot_rows_for_count(block.index_len);
    if slot_cols == 0 || slot_rows == 0 {
        return None;
    }
    let step = slot_step_world();
    let local_x = point[0] - block_left;
    let local_y = block_top - point[1];
    if local_x < 0.0 || local_y < 0.0 {
        return None;
    }

    let slot_col = (local_x / step).floor() as i32;
    let slot_row = (local_y / step).floor() as i32;
    if slot_col < 0 || slot_row < 0 || slot_col >= slot_cols as i32 || slot_row >= slot_rows as i32
    {
        return None;
    }

    let slot_col = slot_col as usize;
    let slot_row = slot_row as usize;
    let (slot_left, slot_bottom, slot_right, slot_top) =
        slot_world_bounds(block_left, block_top, slot_col, slot_row);
    if point[0] < slot_left
        || point[0] > slot_right
        || point[1] < slot_bottom
        || point[1] > slot_top
    {
        return None;
    }

    let local_idx = slot_grid_local_index(slot_col, slot_row, slot_cols);
    if local_idx >= block.index_len {
        return None;
    }

    let idx = block.index_start.saturating_add(local_idx);
    ctx.scene.index_to_id.get(idx).copied()
}

pub(crate) fn media_item_id_at_world_point(ctx: &RenderContext, point: [f64; 2]) -> Option<u64> {
    let id = item_id_at_world_point(ctx, point)?;
    let idx = ctx.scene.index_for_id(id)?;
    if ctx
        .slot_paths
        .get(idx)
        .and_then(|path| path.live_path())
        .is_none()
    {
        return None;
    }

    let raw = ctx.scene.all_items_raw.get(idx).copied()?;
    if !super::slot_has_media_content(raw) {
        return None;
    }

    let (center_x, center_y, width, height) = ctx.scene.item_fitted_world_geometry(idx)?;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    let left = center_x - width as f64 * 0.5;
    let right = center_x + width as f64 * 0.5;
    let bottom = center_y - height as f64 * 0.5;
    let top = center_y + height as f64 * 0.5;
    if point[0] < left || point[0] > right || point[1] < bottom || point[1] > top {
        return None;
    }

    Some(id)
}

fn visible_block_grid_range(ctx: &RenderContext) -> Option<(i32, i32, i32, i32)> {
    let (view_min_x, view_max_x, view_min_y, view_max_y) = ctx.view().viewport_rect();
    let left_edge = ctx.scene.layout_cursor.left_edge as f64;
    let top_edge = ctx.scene.layout_cursor.top_edge as f64;
    let block_span_x = ctx.scene.layout_cursor.grid_cell_w.max(1e-6) as f64;
    let block_span_y = ctx.scene.layout_cursor.grid_cell_h.max(1e-6) as f64;

    let min_col = ((view_min_x - left_edge) / block_span_x).floor() as i32;
    let max_col = ((view_max_x - left_edge) / block_span_x).floor() as i32;
    let min_row = ((top_edge - view_max_y) / block_span_y).floor() as i32;
    let max_row = ((top_edge - view_min_y) / block_span_y).floor() as i32;

    Some((
        min_col.max(0),
        max_col.max(-1),
        min_row.max(0),
        max_row.max(-1),
    ))
}

fn block_grid_for_point(ctx: &RenderContext, point: [f64; 2]) -> Option<BlockGridAddress> {
    let left_edge = ctx.scene.layout_cursor.left_edge as f64;
    let top_edge = ctx.scene.layout_cursor.top_edge as f64;
    let block_span_x = ctx.scene.layout_cursor.grid_cell_w.max(1e-6) as f64;
    let block_span_y = ctx.scene.layout_cursor.grid_cell_h.max(1e-6) as f64;

    let col = ((point[0] - left_edge) / block_span_x).floor() as i32;
    let row = ((top_edge - point[1]) / block_span_y).floor() as i32;
    if col < 0 || row < 0 {
        return None;
    }
    Some(BlockGridAddress { col, row })
}

fn visible_slot_range_in_block(
    view_min_x: f64,
    view_max_x: f64,
    view_min_y: f64,
    view_max_y: f64,
    block_left: f64,
    block_top: f64,
    slot_cols: usize,
    slot_rows: usize,
) -> (usize, usize, usize, usize) {
    let step = slot_step_world();
    let max_col = slot_cols.saturating_sub(1) as i32;
    let max_row = slot_rows.saturating_sub(1) as i32;
    let min_slot_col =
        ((((view_min_x - block_left) / step).floor() as i32) - 1).clamp(0, max_col) as usize;
    let max_slot_col =
        ((((view_max_x - block_left) / step).floor() as i32) + 1).clamp(0, max_col) as usize;
    let min_slot_row =
        ((((block_top - view_max_y) / step).floor() as i32) - 1).clamp(0, max_row) as usize;
    let max_slot_row =
        ((((block_top - view_min_y) / step).floor() as i32) + 1).clamp(0, max_row) as usize;
    (min_slot_col, max_slot_col, min_slot_row, max_slot_row)
}

fn slot_world_bounds(
    block_left: f64,
    block_top: f64,
    slot_col: usize,
    slot_row: usize,
) -> (f64, f64, f64, f64) {
    let step = slot_step_world();
    let left = block_left + slot_col as f64 * step;
    let top = block_top - slot_row as f64 * step;
    let right = left + SLOT_SIDE as f64;
    let bottom = top - SLOT_SIDE as f64;
    (left, bottom, right, top)
}

fn rects_intersect(
    a_left: f64,
    a_bottom: f64,
    a_right: f64,
    a_top: f64,
    b_left: f64,
    b_bottom: f64,
    b_right: f64,
    b_top: f64,
) -> bool {
    a_left <= b_right && a_right >= b_left && a_bottom <= b_top && a_top >= b_bottom
}

fn slot_step_world() -> f64 {
    (SLOT_SIDE + SLOT_GAP) as f64
}
