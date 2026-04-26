use super::placement::{block_slot_for_index, corner_square_grid_items};
use super::plans::build_anchored_block_plan;
use super::BlockLayoutPlanner;
use crate::core::scanner::FileItem;
use crate::render::layout::{
    full_block_content_span, BlockGridAddress, SceneLayoutCursor, BLOCK_FILE_CAP,
    BLOCK_SLOT_COLUMNS, SLOT_GAP, SLOT_SIDE,
};
use std::path::PathBuf;

fn make_file(id: u64) -> FileItem {
    FileItem {
        id,
        asset_key: id + 1,
        path: PathBuf::from(format!("file_{id}.png")),
        width: 100,
        height: 100,
    }
}

fn make_files(count: usize, start_id: u64) -> Vec<FileItem> {
    (0..count)
        .map(|offset| make_file(start_id.saturating_add(offset as u64)))
        .collect()
}

#[test]
fn blocks_are_capped_at_block_file_cap_for_exact_cap_items() {
    let files = make_files(BLOCK_FILE_CAP, 0);
    let mut planner = BlockLayoutPlanner::new(SceneLayoutCursor::new_centered(64.0), 0);
    planner.grow_target_for_total_items(files.len());
    planner.push_many(files.as_slice());
    let batch = planner.finish();

    assert_eq!(batch.blocks.len(), 1);
    assert_eq!(batch.blocks[0].entries.len(), BLOCK_FILE_CAP);
}

#[test]
fn blocks_are_capped_at_block_file_cap_for_cap_plus_one_items() {
    let files = make_files(BLOCK_FILE_CAP + 1, 0);
    let mut planner = BlockLayoutPlanner::new(SceneLayoutCursor::new_centered(64.0), 0);
    planner.grow_target_for_total_items(files.len());
    planner.push_many(files.as_slice());
    let batch = planner.finish();

    assert_eq!(batch.blocks.len(), 2);
    assert_eq!(batch.blocks[0].entries.len(), BLOCK_FILE_CAP);
    assert_eq!(batch.blocks[1].entries.len(), 1);
}

#[test]
fn blocks_are_capped_at_block_file_cap_for_two_full_blocks_plus_one_item() {
    let files = make_files(BLOCK_FILE_CAP * 2 + 1, 0);
    let mut planner = BlockLayoutPlanner::new(SceneLayoutCursor::new_centered(96.0), 0);
    planner.grow_target_for_total_items(files.len());
    planner.push_many(files.as_slice());
    let batch = planner.finish();

    assert_eq!(batch.blocks.len(), 3);
    assert_eq!(batch.blocks[0].entries.len(), BLOCK_FILE_CAP);
    assert_eq!(batch.blocks[1].entries.len(), BLOCK_FILE_CAP);
    assert_eq!(batch.blocks[2].entries.len(), 1);
}

#[test]
fn take_batch_does_not_emit_partial_tail_block() {
    let files = make_files(500, 0);
    let mut planner = BlockLayoutPlanner::new(SceneLayoutCursor::new_centered(64.0), 0);
    planner.grow_target_for_total_items(files.len());
    planner.push_many(files.as_slice());

    let mid = planner.take_batch();
    assert!(mid.blocks.is_empty());

    let finished = planner.finish();
    assert_eq!(finished.blocks.len(), 1);
    assert_eq!(finished.blocks[0].entries.len(), 500);
}

#[test]
fn target_side_only_grows() {
    let mut planner = BlockLayoutPlanner::new(SceneLayoutCursor::new_centered(8.0), 0);
    planner.grow_target_for_total_items(256);
    let first = planner.take_batch().cursor.target_side;

    planner.grow_target_for_total_items(8192);
    let second = planner.take_batch().cursor.target_side;

    planner.grow_target_for_total_items(16);
    let third = planner.take_batch().cursor.target_side;

    assert!(second >= first);
    assert_eq!(third, second);
}

#[test]
fn square_layer_slots_follow_expected_sequence() {
    let expected = [
        (0u64, 0u64),
        (1, 0),
        (0, 1),
        (1, 1),
        (2, 0),
        (2, 1),
        (0, 2),
        (1, 2),
        (2, 2),
    ];
    for (idx, slot) in expected.iter().enumerate() {
        assert_eq!(block_slot_for_index(idx as u64), *slot);
    }
}

#[test]
fn blocks_follow_square_layer_order() {
    let mut planner = BlockLayoutPlanner::new(SceneLayoutCursor::new_centered(8.0), 0);
    let mut bounds = Vec::new();

    for id in 0..5u64 {
        planner.push(make_file(id));
        planner.flush_pending_block();
        let batch = planner.take_batch();
        assert_eq!(batch.blocks.len(), 1);
        bounds.push(batch.blocks[0].bounds);
    }

    let eps = 0.001;
    assert!(bounds[1][0] > bounds[0][0]);
    assert!((bounds[1][3] - bounds[0][3]).abs() < eps);

    assert!((bounds[2][0] - bounds[0][0]).abs() < eps);
    assert!(bounds[2][3] < bounds[0][3]);

    assert!((bounds[3][0] - bounds[1][0]).abs() < eps);
    assert!((bounds[3][3] - bounds[2][3]).abs() < eps);

    assert!(bounds[4][0] > bounds[1][0]);
    assert!((bounds[4][3] - bounds[0][3]).abs() < eps);
}

#[test]
fn block_layout_fills_rows_left_to_right_for_partial_blocks() {
    let files = make_files(34, 0);
    let items = corner_square_grid_items(files.as_slice());
    assert_eq!(items.len(), 34);

    let origin = items[0].data;
    let step = SLOT_SIDE + SLOT_GAP;
    for (idx, item) in items.iter().enumerate() {
        let col = idx % BLOCK_SLOT_COLUMNS;
        let row = idx / BLOCK_SLOT_COLUMNS;
        assert!((item.data[0] - origin[0] - step * col as f32).abs() < 0.001);
        assert!((origin[1] - item.data[1] - step * row as f32).abs() < 0.001);
    }
}

#[test]
fn full_block_uses_fixed_block_footprint() {
    let files = make_files(BLOCK_FILE_CAP, 0);
    let mut planner = BlockLayoutPlanner::new(SceneLayoutCursor::new_centered(64.0), 0);
    planner.push_many(files.as_slice());
    planner.flush_pending_block();
    let batch = planner.take_batch();

    let bounds = batch.blocks[0].bounds;
    let width = bounds[2] - bounds[0];
    let height = bounds[3] - bounds[1];

    assert!((width - full_block_content_span()).abs() < 0.001);
    assert!((height - full_block_content_span()).abs() < 0.001);
}

#[test]
fn anchored_block_plan_aligns_bounds_to_requested_anchor() {
    let files = make_files(2, 0);
    let plan = build_anchored_block_plan(
        files.as_slice(),
        7,
        BlockGridAddress { col: 3, row: 4 },
        10.0,
        20.0,
    )
    .expect("anchored plan");

    let step = SLOT_SIDE + SLOT_GAP;
    assert_eq!(plan.block_id, 7);
    assert_eq!(plan.grid, BlockGridAddress { col: 3, row: 4 });
    assert!((plan.bounds[0] - 10.0).abs() < 0.001);
    assert!((plan.bounds[3] - 20.0).abs() < 0.001);
    assert!((plan.entries[0].raw.data[0] - (10.0 + SLOT_SIDE * 0.5)).abs() < 0.001);
    assert!((plan.entries[0].raw.data[1] - (20.0 - SLOT_SIDE * 0.5)).abs() < 0.001);
    assert!((plan.entries[1].raw.data[0] - plan.entries[0].raw.data[0] - step).abs() < 0.001);
}
