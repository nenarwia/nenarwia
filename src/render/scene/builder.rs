use std::collections::HashMap;
use std::path::PathBuf;

use super::data::{Scene, SceneLayoutBlock};
use crate::render::layout::{
    block_slot_columns_for_count, estimated_layout_extent, slot_grid_position, BlockLayoutPlanner,
    LayoutBlockBatchPlan, SceneLayoutCursor, SlotGridAddress,
};

impl Scene {
    pub fn from_files(files: Vec<crate::core::scanner::FileItem>) -> (Self, Vec<PathBuf>) {
        Self::from_files_slice_with_layout(files.as_slice(), None, None)
    }

    #[allow(dead_code)]
    pub fn from_files_with_layout(
        files: Vec<crate::core::scanner::FileItem>,
        width_override: Option<f32>,
        height_override: Option<f32>,
    ) -> (Self, Vec<PathBuf>) {
        Self::from_files_slice_with_layout(files.as_slice(), width_override, height_override)
    }

    pub fn from_files_slice_with_layout(
        files: &[crate::core::scanner::FileItem],
        width_override: Option<f32>,
        height_override: Option<f32>,
    ) -> (Self, Vec<PathBuf>) {
        Self::from_files_slice_with_layout_aligned(files, width_override, height_override)
    }

    fn from_files_slice_with_layout_aligned(
        files: &[crate::core::scanner::FileItem],
        width_override: Option<f32>,
        height_override: Option<f32>,
    ) -> (Self, Vec<PathBuf>) {
        let target_side = width_override
            .or(height_override)
            .unwrap_or_else(|| estimated_layout_extent(files.len()))
            .max(8.0);
        let mut planner = BlockLayoutPlanner::new(SceneLayoutCursor::new_centered(target_side), 0);
        planner.grow_target_for_total_items(files.len());
        planner.push_many(files);
        let batch = planner.finish();

        Self::from_layout_batch(batch)
    }

    pub fn from_layout_batch(batch: LayoutBlockBatchPlan) -> (Self, Vec<PathBuf>) {
        let mut layout_blocks = Vec::with_capacity(batch.blocks.len());
        let mut all_items_raw = Vec::new();
        let mut slot_addresses = Vec::new();
        let mut file_paths = Vec::new();
        let mut block_grid_lookup = HashMap::new();
        let mut id_to_index = HashMap::new();
        let mut index_to_id = Vec::new();
        let mut asset_keys = Vec::new();
        let mut asset_key_to_index = HashMap::new();
        let mut quality_debt = Vec::new();
        let mut item_dimensions = Vec::new();
        let mut last_lod = Vec::new();
        let mut display_lod = Vec::new();
        let mut render_lod = Vec::new();
        let mut coarse_lod = Vec::new();

        let mut total_count = 0usize;
        for planned_block in batch.blocks {
            let block_start = total_count;
            let block_grid = planned_block.grid;
            let slot_cols = block_slot_columns_for_count(planned_block.entries.len());

            block_grid_lookup.insert(block_grid, layout_blocks.len());

            for (local_idx, entry) in planned_block.entries.into_iter().enumerate() {
                let idx = total_count;
                let id = entry.file.id;
                let asset_key = entry.file.asset_key;
                let dims = (entry.file.width, entry.file.height);

                id_to_index.insert(id, idx);
                index_to_id.push(id);
                asset_keys.push(asset_key);
                asset_key_to_index.insert(asset_key, idx);
                item_dimensions.push(dims);
                quality_debt.push(0.0);
                last_lod.push(0);
                display_lod.push(u8::MAX);
                render_lod.push(u8::MAX);
                coarse_lod.push(u8::MAX);
                all_items_raw.push(entry.raw);
                let (col, row) = slot_grid_position(local_idx, slot_cols);
                slot_addresses.push(SlotGridAddress {
                    block: block_grid,
                    col: col as u8,
                    row: row as u8,
                });
                file_paths.push(entry.file.path);
                total_count = total_count.saturating_add(1);
            }

            let block_len = total_count.saturating_sub(block_start);
            if block_len == 0 {
                continue;
            }

            layout_blocks.push(SceneLayoutBlock {
                block_id: planned_block.block_id,
                grid: block_grid,
                bounds: planned_block.bounds,
                index_start: block_start,
                index_len: block_len,
            });
        }

        let mut scene = Self {
            layout_blocks,
            layout_cursor: batch.cursor,
            all_items_raw,
            slot_addresses,
            total_count,
            layout_width: batch.layout_width,
            layout_height: batch.layout_height,
            block_grid_lookup,
            id_to_index,
            index_to_id,
            asset_keys,
            asset_key_to_index,
            quality_debt,
            item_dimensions,
            last_lod,
            display_lod,
            render_lod,
            coarse_lod,
        };
        scene.refresh_layout_extent_from_blocks();
        (scene, file_paths)
    }
}

#[cfg(test)]
mod tests {
    use super::Scene;
    use crate::core::scanner::FileItem;
    use std::path::PathBuf;

    fn make_file(id: u64) -> FileItem {
        FileItem {
            id,
            asset_key: id.saturating_add(1),
            path: PathBuf::from(format!("builder_file_{id}.png")),
            width: 100,
            height: 100,
        }
    }

    #[test]
    fn scene_tracks_ids_beyond_u32_limit() {
        let high_id = u32::MAX as u64 + 42;
        let files = vec![make_file(high_id), make_file(high_id + 1)];

        let (scene, paths) = Scene::from_files(files);

        assert_eq!(scene.total_count, 2);
        assert_eq!(paths.len(), 2);
        assert_eq!(scene.index_for_id(high_id), Some(0));
        assert_eq!(scene.index_for_id(high_id + 1), Some(1));
        assert_eq!(scene.index_to_id, vec![high_id, high_id + 1]);
    }
}
