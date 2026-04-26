use super::super::types::{block_slot_columns_for_count, slot_grid_position, SLOT_GAP, SLOT_SIDE};
use crate::core::scanner::FileItem;
use crate::render::instance::InstanceRaw;

pub(super) fn corner_square_grid_items(files: &[FileItem]) -> Vec<InstanceRaw> {
    let mut items = Vec::with_capacity(files.len());
    let slot_cols = block_slot_columns_for_count(files.len());
    for (local_idx, file) in files.iter().enumerate() {
        let (x, y) = anchored_slot_center(0.0, 0.0, local_idx, slot_cols);
        items.push(build_item_instance(file, x, y, SLOT_SIDE, SLOT_SIDE));
    }
    items
}

pub(super) fn raw_items_bounds(items: &[InstanceRaw]) -> Option<[f32; 4]> {
    if items.is_empty() {
        return None;
    }

    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for raw in items {
        let half_w = raw.data[2].max(0.0) * 0.5;
        let half_h = raw.data[3].max(0.0) * 0.5;
        min_x = min_x.min(raw.data[0] - half_w);
        min_y = min_y.min(raw.data[1] - half_h);
        max_x = max_x.max(raw.data[0] + half_w);
        max_y = max_y.max(raw.data[1] + half_h);
    }

    if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
        return None;
    }

    Some([min_x, min_y, max_x, max_y])
}

pub(super) fn anchored_slot_center(
    left: f32,
    top: f32,
    local_idx: usize,
    slot_cols: usize,
) -> (f32, f32) {
    let (col, row) = slot_grid_position(local_idx, slot_cols);
    let slot_step = SLOT_SIDE + SLOT_GAP;
    let x = left + col as f32 * slot_step + SLOT_SIDE * 0.5;
    let y = top - row as f32 * slot_step - SLOT_SIDE * 0.5;
    (x, y)
}

pub(super) fn build_item_instance(file: &FileItem, x: f32, y: f32, w: f32, h: f32) -> InstanceRaw {
    InstanceRaw {
        data: [x, y, w, h],
        color: [0.5, 0.5, 0.5, 1.0],
        uv_region: [0.0, 0.0, 0.0, 0.0],
        params: [-1.0, -1.0, 0.0, 0.0],
        params2: [-1.0, -1.0, 0.0, 0.0],
        sample_flags: [0.0, 0.0, 0.0, 0.0],
        fit_rect: InstanceRaw::contain_fit_rect(file.width, file.height),
    }
}

pub(super) fn block_slot_for_index(idx: u64) -> (u64, u64) {
    if idx == 0 {
        return (0, 0);
    }

    let mut side = (idx as f64).sqrt().floor() as u64;
    while (side as u128 + 1).saturating_mul(side as u128 + 1) <= idx as u128 {
        side = side.saturating_add(1);
    }
    while (side as u128).saturating_mul(side as u128) > idx as u128 {
        side = side.saturating_sub(1);
    }

    let offset = idx.saturating_sub(side.saturating_mul(side));
    if offset < side {
        return (side, offset);
    }
    if offset < side.saturating_mul(2) {
        return (offset.saturating_sub(side), side);
    }
    (side, side)
}
