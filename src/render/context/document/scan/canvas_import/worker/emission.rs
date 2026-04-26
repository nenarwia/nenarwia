use std::sync::mpsc;

use crate::render::context::document::{RestoredSlot, ScanResult};
use crate::render::layout::{LayoutBlockBatchPlan, LayoutBlockPlan, SceneLayoutCursor};
use crate::render::scene::{SceneAppendBlocksBatch, SceneTailPlan};

use super::super::tail_refill::{fixed_grid_cursor_for_append, merge_bounds_opt, TailRefillState};

pub(super) fn send_planned_append(
    restores: Vec<RestoredSlot>,
    planner_batch: LayoutBlockBatchPlan,
    tail_plan: Option<LayoutBlockPlan>,
    tail_append_plan: Option<crate::render::layout::TailAppendPlan>,
    tail_state: Option<&TailRefillState>,
    start_cursor: SceneLayoutCursor,
    start_bounds: Option<[f32; 4]>,
    epoch: u64,
    tab_id: u64,
    document_revision: u64,
    tx: &mpsc::Sender<ScanResult>,
    final_scan: bool,
) -> bool {
    if restores.is_empty()
        && tail_plan.is_none()
        && tail_append_plan.is_none()
        && planner_batch.is_empty()
    {
        return true;
    }

    let (layout_width, layout_height, layout_cursor) = resolve_emission_layout(
        &planner_batch,
        tail_plan.as_ref(),
        tail_append_plan.as_ref(),
        tail_state,
        start_cursor,
        start_bounds,
    );

    send_append_snapshot(
        restores,
        SceneAppendBlocksBatch {
            layout_width,
            layout_height,
            layout_cursor,
            tail: tail_plan
                .map(SceneTailPlan::Refill)
                .or_else(|| tail_append_plan.map(SceneTailPlan::Append)),
            blocks: planner_batch.blocks,
        },
        epoch,
        tab_id,
        document_revision,
        tx,
        final_scan,
    )
}

fn resolve_emission_layout(
    planner_batch: &LayoutBlockBatchPlan,
    tail_plan: Option<&LayoutBlockPlan>,
    tail_append_plan: Option<&crate::render::layout::TailAppendPlan>,
    tail_state: Option<&TailRefillState>,
    start_cursor: SceneLayoutCursor,
    start_bounds: Option<[f32; 4]>,
) -> (f32, f32, SceneLayoutCursor) {
    if !planner_batch.is_empty() {
        return (
            planner_batch.layout_width,
            planner_batch.layout_height,
            planner_batch.cursor,
        );
    }

    if let Some(tail_state) = tail_state {
        let refill_bounds = tail_plan
            .map(|plan| plan.bounds)
            .unwrap_or(tail_state.seed_bounds());
        let merged = merge_bounds_opt(start_bounds, refill_bounds);
        let width = merged
            .map(|bounds| (bounds[2] - bounds[0]).max(0.0))
            .unwrap_or(start_cursor.target_side)
            .max(start_cursor.target_side);
        let height = merged
            .map(|bounds| (bounds[3] - bounds[1]).max(0.0))
            .unwrap_or(start_cursor.target_side)
            .max(start_cursor.target_side);
        return (width, height, fixed_grid_cursor_for_append(start_cursor));
    }

    if let Some(tail_append_plan) = tail_append_plan {
        let merged = merge_bounds_opt(start_bounds, tail_append_plan.bounds);
        let width = merged
            .map(|bounds| (bounds[2] - bounds[0]).max(0.0))
            .unwrap_or(start_cursor.target_side)
            .max(start_cursor.target_side);
        let height = merged
            .map(|bounds| (bounds[3] - bounds[1]).max(0.0))
            .unwrap_or(start_cursor.target_side)
            .max(start_cursor.target_side);
        return (width, height, fixed_grid_cursor_for_append(start_cursor));
    }

    (
        start_cursor.target_side,
        start_cursor.target_side,
        start_cursor,
    )
}

pub(super) fn empty_batch_for_cursor(cursor: SceneLayoutCursor) -> LayoutBlockBatchPlan {
    LayoutBlockBatchPlan {
        blocks: Vec::new(),
        cursor,
        layout_width: cursor.target_side,
        layout_height: cursor.target_side,
    }
}

pub(super) fn send_append_snapshot(
    restores: Vec<RestoredSlot>,
    batch: SceneAppendBlocksBatch,
    epoch: u64,
    tab_id: u64,
    document_revision: u64,
    tx: &mpsc::Sender<ScanResult>,
    final_scan: bool,
) -> bool {
    tx.send(ScanResult {
        epoch,
        tab_id,
        document_revision,
        restores,
        batch,
        final_scan,
    })
    .is_ok()
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use crate::core::scanner::FileItem;
    use crate::render::instance::InstanceRaw;
    use crate::render::layout::{
        BlockGridAddress, LayoutBlockBatchPlan, LayoutBlockEntry, LayoutBlockPlan,
        SceneLayoutCursor, TailAppendPlan,
    };
    use crate::render::scene::SceneAppendBlocksBatch;

    use crate::render::context::document::scan::canvas_import::tail_refill::{
        TailRefillSeed, TailRefillState,
    };

    use super::{
        empty_batch_for_cursor, fixed_grid_cursor_for_append, resolve_emission_layout,
        send_append_snapshot,
    };

    #[test]
    fn empty_batch_uses_cursor_target_side() {
        let cursor = SceneLayoutCursor::new_centered(8.0);
        let batch = empty_batch_for_cursor(cursor);

        assert!(batch.blocks.is_empty());
        assert_eq!(batch.layout_width, cursor.target_side);
        assert_eq!(batch.layout_height, cursor.target_side);
    }

    #[test]
    fn planner_batch_layout_wins() {
        let cursor = SceneLayoutCursor::new_centered(8.0);
        let batch = LayoutBlockBatchPlan {
            blocks: vec![LayoutBlockPlan {
                block_id: 1,
                grid: BlockGridAddress { col: 0, row: 0 },
                bounds: [0.0, 0.0, 1.0, 1.0],
                entries: vec![LayoutBlockEntry {
                    raw: InstanceRaw {
                        data: [0.0; 4],
                        color: [0.0; 4],
                        uv_region: [0.0; 4],
                        params: [0.0; 4],
                        params2: [0.0; 4],
                        sample_flags: [0.0; 4],
                        fit_rect: [0.0; 4],
                    },
                    file: FileItem {
                        id: 1,
                        asset_key: 1,
                        path: "planner.png".into(),
                        width: 1,
                        height: 1,
                    },
                }],
            }],
            cursor,
            layout_width: 123.0,
            layout_height: 456.0,
        };

        let (width, height, _) = resolve_emission_layout(&batch, None, None, None, cursor, None);
        assert_eq!(width, 123.0);
        assert_eq!(height, 456.0);
    }

    #[test]
    fn tail_refill_layout_uses_merged_bounds_and_fixed_cursor() {
        let cursor = SceneLayoutCursor::new_centered(8.0);
        let refill = TailRefillState::new(TailRefillSeed {
            block_id: 1,
            grid: BlockGridAddress { col: 0, row: 0 },
            existing_len: 1,
            bounds: [0.0, -10.0, 10.0, 0.0],
            files: vec![FileItem {
                id: 1,
                asset_key: 1,
                path: "seed.png".into(),
                width: 1,
                height: 1,
            }],
        });

        let (width, height, layout_cursor) = resolve_emission_layout(
            &LayoutBlockBatchPlan {
                blocks: Vec::new(),
                cursor,
                layout_width: cursor.target_side,
                layout_height: cursor.target_side,
            },
            None,
            None,
            Some(&refill),
            cursor,
            Some([-2.0, -2.0, 2.0, 2.0]),
        );

        assert_eq!(width, 12.0);
        assert_eq!(height, 12.0);
        assert_eq!(layout_cursor, fixed_grid_cursor_for_append(cursor));
    }

    #[test]
    fn tail_append_layout_uses_merged_bounds_and_fixed_cursor() {
        let cursor = SceneLayoutCursor::new_centered(8.0);
        let tail_append = TailAppendPlan {
            block_id: 1,
            grid: BlockGridAddress { col: 0, row: 0 },
            start_local_idx: 2,
            bounds: [0.0, -9.0, 11.0, 0.0],
            entries: Vec::new(),
        };

        let (width, height, layout_cursor) = resolve_emission_layout(
            &LayoutBlockBatchPlan {
                blocks: Vec::new(),
                cursor,
                layout_width: cursor.target_side,
                layout_height: cursor.target_side,
            },
            None,
            Some(&tail_append),
            None,
            cursor,
            Some([-3.0, -1.0, 2.0, 2.0]),
        );

        assert_eq!(width, 14.0);
        assert_eq!(height, 11.0);
        assert_eq!(layout_cursor, fixed_grid_cursor_for_append(cursor));
    }

    #[test]
    fn closed_channel_returns_false() {
        let (tx, rx) = mpsc::channel();
        drop(rx);

        let ok = send_append_snapshot(
            Vec::new(),
            SceneAppendBlocksBatch {
                layout_width: 1.0,
                layout_height: 1.0,
                layout_cursor: SceneLayoutCursor::new_centered(8.0),
                tail: None,
                blocks: Vec::new(),
            },
            1,
            1,
            1,
            &tx,
            true,
        );

        assert!(!ok);
    }
}
