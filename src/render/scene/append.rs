use crate::render::instance::InstanceRaw;
use crate::render::layout::{
    block_slot_columns_for_count, slot_grid_position, BlockGridAddress, LayoutBlockPlan,
    SceneLayoutCursor, SlotGridAddress, TailAppendPlan, SLOT_GAP, SLOT_SIDE,
};

use super::data::{Scene, SceneLayoutBlock};

#[derive(Clone, Debug)]
pub enum SceneTailPlan {
    Refill(LayoutBlockPlan),
    Append(TailAppendPlan),
}

#[derive(Clone, Debug)]
pub struct SceneAppendBlocksBatch {
    pub layout_width: f32,
    pub layout_height: f32,
    pub layout_cursor: SceneLayoutCursor,
    pub tail: Option<SceneTailPlan>,
    pub blocks: Vec<LayoutBlockPlan>,
}

impl SceneAppendBlocksBatch {
    pub fn is_empty(&self) -> bool {
        self.tail.is_none() && self.blocks.is_empty()
    }
}

impl Scene {
    pub fn append_batch(&mut self, batch: SceneAppendBlocksBatch) {
        let SceneAppendBlocksBatch {
            layout_width,
            layout_height,
            layout_cursor,
            tail,
            blocks,
        } = batch;
        if tail.is_none() && blocks.is_empty() {
            return;
        }

        self.layout_cursor = layout_cursor;
        self.layout_cursor
            .grow_target_side(layout_width.max(layout_height));
        self.layout_width = self.layout_width.max(layout_width);
        self.layout_height = self.layout_height.max(layout_height);

        if let Some(plan) = tail {
            match plan {
                SceneTailPlan::Refill(plan) => self.apply_tail_refill_block(plan),
                SceneTailPlan::Append(plan) => self.apply_tail_append_block(plan),
            }
        }

        for planned_block in blocks {
            let block_start = self.total_count;
            let additional = planned_block.entries.len();
            let slot_cols = block_slot_columns_for_count(additional);
            self.all_items_raw.reserve(additional);
            self.slot_addresses.reserve(additional);
            self.index_to_id.reserve(additional);
            self.asset_keys.reserve(additional);
            self.item_dimensions.reserve(additional);
            self.quality_debt.reserve(additional);
            self.last_lod.reserve(additional);
            self.display_lod.reserve(additional);
            self.render_lod.reserve(additional);
            self.coarse_lod.reserve(additional);

            let block_grid = planned_block.grid;
            self.block_grid_lookup
                .insert(block_grid, self.layout_blocks.len());
            for (local_idx, entry) in planned_block.entries.into_iter().enumerate() {
                let idx = self.total_count;
                let id = entry.file.id;
                let asset_key = entry.file.asset_key;
                let dims = (entry.file.width, entry.file.height);

                self.id_to_index.insert(id, idx);
                self.index_to_id.push(id);
                self.asset_keys.push(asset_key);
                self.asset_key_to_index.insert(asset_key, idx);
                self.item_dimensions.push(dims);
                self.quality_debt.push(0.0);
                self.last_lod.push(0);
                self.display_lod.push(u8::MAX);
                self.render_lod.push(u8::MAX);
                self.coarse_lod.push(u8::MAX);
                self.all_items_raw.push(entry.raw);
                self.slot_addresses
                    .push(slot_address_for_local_idx(block_grid, local_idx, slot_cols));
                self.total_count = self.total_count.saturating_add(1);
            }

            if additional == 0 {
                continue;
            }

            self.layout_blocks.push(SceneLayoutBlock {
                block_id: planned_block.block_id,
                grid: block_grid,
                bounds: planned_block.bounds,
                index_start: block_start,
                index_len: additional,
            });
        }

        self.refresh_layout_extent_from_blocks();
    }

    fn apply_tail_refill_block(&mut self, planned_block: LayoutBlockPlan) {
        let Some(last_pos) = self
            .layout_blocks
            .iter()
            .position(|block| block.block_id == planned_block.block_id)
        else {
            return;
        };
        if last_pos + 1 != self.layout_blocks.len() {
            return;
        }

        let block = &mut self.layout_blocks[last_pos];
        let block_grid = block.grid;
        let old_len = block.index_len;
        let start = block.index_start;
        let new_len = planned_block.entries.len();
        let slot_cols = block_slot_columns_for_count(new_len);
        if new_len < old_len {
            return;
        }

        for (local_idx, entry) in planned_block.entries.into_iter().enumerate() {
            let target_idx = if local_idx < old_len {
                start.saturating_add(local_idx)
            } else {
                self.total_count
                    .saturating_add(local_idx.saturating_sub(old_len))
            };

            let id = entry.file.id;
            let asset_key = entry.file.asset_key;
            let dims = (entry.file.width, entry.file.height);

            if local_idx < old_len {
                if target_idx >= self.all_items_raw.len()
                    || target_idx >= self.index_to_id.len()
                    || target_idx >= self.asset_keys.len()
                    || target_idx >= self.item_dimensions.len()
                {
                    return;
                }

                let prev_id = self.index_to_id[target_idx];
                let prev_raw = self.all_items_raw[target_idx];
                let mut next_raw = entry.raw;
                let preserve_runtime_state =
                    prev_id == id && self.asset_keys[target_idx] == asset_key;
                if preserve_runtime_state {
                    next_raw.color = prev_raw.color;
                    next_raw.uv_region = prev_raw.uv_region;
                    next_raw.params = prev_raw.params;
                    next_raw.params2 = prev_raw.params2;
                    next_raw.sample_flags = prev_raw.sample_flags;
                }
                self.all_items_raw[target_idx] = next_raw;

                if prev_id != id {
                    self.id_to_index.remove(&prev_id);
                    self.id_to_index.insert(id, target_idx);
                    self.index_to_id[target_idx] = id;
                }

                let prev_asset = self.asset_keys[target_idx];
                if prev_asset != asset_key {
                    self.asset_key_to_index.remove(&prev_asset);
                    self.asset_key_to_index.insert(asset_key, target_idx);
                    self.asset_keys[target_idx] = asset_key;
                }

                self.item_dimensions[target_idx] = dims;
            } else {
                self.id_to_index.insert(id, target_idx);
                self.index_to_id.push(id);
                self.asset_keys.push(asset_key);
                self.asset_key_to_index.insert(asset_key, target_idx);
                self.item_dimensions.push(dims);
                self.quality_debt.push(0.0);
                self.last_lod.push(0);
                self.display_lod.push(u8::MAX);
                self.render_lod.push(u8::MAX);
                self.coarse_lod.push(u8::MAX);
                self.all_items_raw.push(entry.raw);
                self.slot_addresses
                    .push(slot_address_for_local_idx(block_grid, local_idx, slot_cols));
            }

            if target_idx < self.slot_addresses.len() {
                self.slot_addresses[target_idx] =
                    slot_address_for_local_idx(block_grid, local_idx, slot_cols);
            }
        }

        let added = new_len.saturating_sub(old_len);
        self.total_count = self.total_count.saturating_add(added);

        block.grid = planned_block.grid;
        block.bounds = planned_block.bounds;
        block.index_len = new_len;
    }

    fn apply_tail_append_block(&mut self, planned_block: TailAppendPlan) {
        let Some(last_pos) = self
            .layout_blocks
            .iter()
            .position(|block| block.block_id == planned_block.block_id)
        else {
            return;
        };
        if last_pos + 1 != self.layout_blocks.len() {
            return;
        }

        let block = &mut self.layout_blocks[last_pos];
        if planned_block.start_local_idx != block.index_len {
            return;
        }

        let block_grid = block.grid;
        let start_local_idx = planned_block.start_local_idx;
        let added = planned_block.entries.len();
        let new_len = block.index_len.saturating_add(added);
        let slot_cols = block_slot_columns_for_count(new_len);
        if added == 0 {
            return;
        }

        self.all_items_raw.reserve(added);
        self.slot_addresses.reserve(added);
        self.index_to_id.reserve(added);
        self.asset_keys.reserve(added);
        self.item_dimensions.reserve(added);
        self.quality_debt.reserve(added);
        self.last_lod.reserve(added);
        self.display_lod.reserve(added);
        self.render_lod.reserve(added);
        self.coarse_lod.reserve(added);

        for local_idx in 0..start_local_idx {
            let target_idx = block.index_start.saturating_add(local_idx);
            let Some(raw) = self.all_items_raw.get_mut(target_idx) else {
                return;
            };
            let Some(address) = self.slot_addresses.get_mut(target_idx) else {
                return;
            };
            reposition_existing_slot(
                raw,
                address,
                block_grid,
                planned_block.bounds[0],
                planned_block.bounds[3],
                local_idx,
                slot_cols,
            );
        }

        for (offset, entry) in planned_block.entries.into_iter().enumerate() {
            let local_idx = start_local_idx.saturating_add(offset);
            let idx = self.total_count;
            let id = entry.file.id;
            let asset_key = entry.file.asset_key;
            let dims = (entry.file.width, entry.file.height);

            self.id_to_index.insert(id, idx);
            self.index_to_id.push(id);
            self.asset_keys.push(asset_key);
            self.asset_key_to_index.insert(asset_key, idx);
            self.item_dimensions.push(dims);
            self.quality_debt.push(0.0);
            self.last_lod.push(0);
            self.display_lod.push(u8::MAX);
            self.render_lod.push(u8::MAX);
            self.coarse_lod.push(u8::MAX);
            self.all_items_raw.push(entry.raw);
            self.slot_addresses
                .push(slot_address_for_local_idx(block_grid, local_idx, slot_cols));
            self.total_count = self.total_count.saturating_add(1);
        }

        block.grid = planned_block.grid;
        block.bounds = planned_block.bounds;
        block.index_len = new_len;
    }
}

fn slot_address_for_local_idx(
    block: BlockGridAddress,
    local_idx: usize,
    slot_cols: usize,
) -> SlotGridAddress {
    let (col, row) = slot_grid_position(local_idx, slot_cols);
    SlotGridAddress {
        block,
        col: col as u8,
        row: row as u8,
    }
}

fn reposition_existing_slot(
    raw: &mut InstanceRaw,
    address: &mut SlotGridAddress,
    block: BlockGridAddress,
    block_left: f32,
    block_top: f32,
    local_idx: usize,
    slot_cols: usize,
) {
    let (col, row) = slot_grid_position(local_idx, slot_cols);
    let step = SLOT_SIDE + SLOT_GAP;
    raw.data[0] = block_left + col as f32 * step + SLOT_SIDE * 0.5;
    raw.data[1] = block_top - row as f32 * step - SLOT_SIDE * 0.5;
    *address = SlotGridAddress {
        block,
        col: col as u8,
        row: row as u8,
    };
}

#[cfg(test)]
mod tests {
    use super::{Scene, SceneAppendBlocksBatch, SceneTailPlan};
    use crate::core::scanner::FileItem;
    use crate::render::layout::{
        append_files_to_block_tail_at_anchor, relayout_block_at_anchor, BlockLayoutPlanner,
        BLOCK_SLOT_COLUMNS,
    };
    use std::path::PathBuf;

    fn make_file(id: u64) -> FileItem {
        FileItem {
            id,
            asset_key: id + 1,
            path: PathBuf::from(format!("scene_file_{id}.png")),
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
    fn append_keeps_existing_block_bounds_stable_and_only_grows_target() {
        let base_files = make_files(1300, 0);
        let (mut scene, _) = Scene::from_files(base_files);
        let old_bounds: Vec<[f32; 4]> = scene.layout_blocks.iter().map(|b| b.bounds).collect();
        let old_target = scene.layout_cursor.target_side;

        let append_files = make_files(4000, 1300);
        let mut planner = BlockLayoutPlanner::new_with_seed_bounds(
            scene.layout_cursor,
            scene.next_block_id(),
            scene.layout_bounds(),
        );
        planner.grow_target_for_total_items(scene.total_count.saturating_add(append_files.len()));
        planner.push_many(append_files.as_slice());
        let planned = planner.finish();

        scene.append_batch(SceneAppendBlocksBatch {
            layout_width: planned.layout_width,
            layout_height: planned.layout_height,
            layout_cursor: planned.cursor,
            tail: None,
            blocks: planned.blocks,
        });

        assert!(
            scene.layout_cursor.target_side >= old_target,
            "target side must not shrink"
        );
        assert!(scene.layout_blocks.len() > old_bounds.len());
        for (idx, before) in old_bounds.iter().enumerate() {
            assert_eq!(*before, scene.layout_blocks[idx].bounds);
        }
    }

    #[test]
    fn tail_refill_updates_last_block_and_appends_only_new_tail_entries() {
        let base_files = make_files(1029, 0);
        let (mut scene, _) = Scene::from_files(base_files.clone());
        let old_total = scene.total_count;
        let old_block_count = scene.layout_blocks.len();
        let tail = scene.layout_blocks.last().expect("tail block exists");
        let tail_len = tail.index_len;
        let tail_block_id = tail.block_id;
        let tail_start = tail.index_start;
        let tail_left = tail.bounds[0];
        let tail_top = tail.bounds[3];

        let mut combined = Vec::new();
        combined.extend_from_slice(&base_files[tail_start..tail_start.saturating_add(tail_len)]);
        combined.extend(make_files(10, 1029));
        let refill = relayout_block_at_anchor(
            combined.as_slice(),
            tail_block_id,
            tail.grid,
            tail_left,
            tail_top,
        )
        .expect("tail relayout");

        scene.append_batch(SceneAppendBlocksBatch {
            layout_width: scene.layout_width,
            layout_height: scene.layout_height,
            layout_cursor: scene.layout_cursor,
            tail: Some(SceneTailPlan::Refill(refill)),
            blocks: Vec::new(),
        });

        assert_eq!(scene.layout_blocks.len(), old_block_count);
        assert_eq!(
            scene.layout_blocks.last().expect("tail").index_len,
            tail_len.saturating_add(10)
        );
        assert_eq!(scene.total_count, old_total.saturating_add(10));
    }

    #[test]
    fn tail_refill_preserves_existing_runtime_state_for_unchanged_entries() {
        let base_files = make_files(1029, 0);
        let (mut scene, _) = Scene::from_files(base_files.clone());
        let tail = scene.layout_blocks.last().expect("tail block exists");
        let tail_len = tail.index_len;
        let tail_block_id = tail.block_id;
        let tail_start = tail.index_start;
        let tail_left = tail.bounds[0];
        let tail_top = tail.bounds[3];

        scene.all_items_raw[tail_start].uv_region = [2.25, 0.1, 0.2, 0.3];
        scene.all_items_raw[tail_start].params = [1.0, 2.0, 3.0, 4.0];
        scene.all_items_raw[tail_start].params2 = [5.0, 6.0, 7.0, 8.0];
        scene.all_items_raw[tail_start].sample_flags = [9.0, 10.0, 11.0, 12.0];
        scene.all_items_raw[tail_start].color = [0.1, 0.2, 0.3, 0.4];

        let mut combined = Vec::new();
        combined.extend_from_slice(&base_files[tail_start..tail_start.saturating_add(tail_len)]);
        combined.extend(make_files(10, 1029));
        let refill = relayout_block_at_anchor(
            combined.as_slice(),
            tail_block_id,
            tail.grid,
            tail_left,
            tail_top,
        )
        .expect("tail relayout");

        scene.append_batch(SceneAppendBlocksBatch {
            layout_width: scene.layout_width,
            layout_height: scene.layout_height,
            layout_cursor: scene.layout_cursor,
            tail: Some(SceneTailPlan::Refill(refill)),
            blocks: Vec::new(),
        });

        let raw = scene.all_items_raw[tail_start];
        assert_eq!(raw.uv_region, [2.25, 0.1, 0.2, 0.3]);
        assert_eq!(raw.params, [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(raw.params2, [5.0, 6.0, 7.0, 8.0]);
        assert_eq!(raw.sample_flags, [9.0, 10.0, 11.0, 12.0]);
        assert_eq!(raw.color, [0.1, 0.2, 0.3, 0.4]);
    }

    #[test]
    fn tail_append_grows_last_block_without_relaying_existing_entries() {
        let base_files = make_files(1023, 0);
        let (mut scene, _) = Scene::from_files(base_files.clone());
        let old_total = scene.total_count;
        let tail = scene.layout_blocks.last().expect("tail block exists");
        let tail_block_id = tail.block_id;
        let tail_grid = tail.grid;
        let tail_len = tail.index_len;
        let tail_bounds = tail.bounds;
        let original_first_raw = scene.all_items_raw[0];

        let appended_files = make_files(1, 5000);
        let append_plan = append_files_to_block_tail_at_anchor(
            appended_files.as_slice(),
            tail_block_id,
            tail_grid,
            tail_bounds[0],
            tail_bounds[3],
            tail_len,
        )
        .expect("tail append plan");

        scene.append_batch(SceneAppendBlocksBatch {
            layout_width: scene.layout_width,
            layout_height: scene.layout_height,
            layout_cursor: scene.layout_cursor,
            tail: Some(SceneTailPlan::Append(append_plan)),
            blocks: Vec::new(),
        });

        let tail_after = scene.layout_blocks.last().expect("tail");
        assert_eq!(scene.total_count, old_total.saturating_add(1));
        assert_eq!(tail_after.block_id, tail_block_id);
        assert_eq!(tail_after.index_len, tail_len.saturating_add(1));
        assert_eq!(scene.index_to_id.last().copied(), Some(5000));
        assert_eq!(
            scene.slot_addresses.last().expect("slot").row as usize,
            tail_len / BLOCK_SLOT_COLUMNS
        );
        assert_eq!(
            scene.slot_addresses.last().expect("slot").col as usize,
            tail_len % BLOCK_SLOT_COLUMNS
        );
        assert_eq!(scene.all_items_raw[0], original_first_raw);
    }

    #[test]
    fn tail_append_keeps_existing_slots_stable_while_growing_block() {
        let base_files = make_files(4, 0);
        let (mut scene, _) = Scene::from_files(base_files.clone());
        let tail = scene.layout_blocks.last().expect("tail block exists");
        let tail_block_id = tail.block_id;
        let tail_grid = tail.grid;
        let tail_len = tail.index_len;
        let tail_bounds = tail.bounds;

        let before_fourth = scene.slot_addresses[3];
        assert_eq!(before_fourth.col, 3);
        assert_eq!(before_fourth.row, 0);

        let appended_files = make_files(1, 9000);
        let append_plan = append_files_to_block_tail_at_anchor(
            appended_files.as_slice(),
            tail_block_id,
            tail_grid,
            tail_bounds[0],
            tail_bounds[3],
            tail_len,
        )
        .expect("tail append plan");

        scene.append_batch(SceneAppendBlocksBatch {
            layout_width: scene.layout_width,
            layout_height: scene.layout_height,
            layout_cursor: scene.layout_cursor,
            tail: Some(SceneTailPlan::Append(append_plan)),
            blocks: Vec::new(),
        });

        let moved_fourth = scene.slot_addresses[3];
        let appended = scene.slot_addresses[4];
        assert_eq!(moved_fourth.col, 3);
        assert_eq!(moved_fourth.row, 0);
        assert_eq!(appended.col, 4);
        assert_eq!(appended.row, 0);
        assert_eq!(before_fourth, moved_fourth);
    }

    #[test]
    fn tail_refill_keeps_existing_slots_stable_while_growing_block() {
        let base_files = make_files(4, 0);
        let (mut scene, _) = Scene::from_files(base_files.clone());
        let tail = scene.layout_blocks.last().expect("tail block exists");
        let tail_block_id = tail.block_id;
        let tail_start = tail.index_start;
        let tail_left = tail.bounds[0];
        let tail_top = tail.bounds[3];

        let before_fourth = scene.slot_addresses[tail_start + 3];
        assert_eq!(before_fourth.col, 3);
        assert_eq!(before_fourth.row, 0);

        let mut combined = Vec::new();
        combined.extend_from_slice(&base_files[tail_start..tail_start + 4]);
        combined.extend(make_files(1, 7000));
        let refill = relayout_block_at_anchor(
            combined.as_slice(),
            tail_block_id,
            tail.grid,
            tail_left,
            tail_top,
        )
        .expect("tail relayout");

        scene.append_batch(SceneAppendBlocksBatch {
            layout_width: scene.layout_width,
            layout_height: scene.layout_height,
            layout_cursor: scene.layout_cursor,
            tail: Some(SceneTailPlan::Refill(refill)),
            blocks: Vec::new(),
        });

        let moved_fourth = scene.slot_addresses[tail_start + 3];
        let appended = scene.slot_addresses[tail_start + 4];
        assert_eq!(moved_fourth.col, 3);
        assert_eq!(moved_fourth.row, 0);
        assert_eq!(appended.col, 4);
        assert_eq!(appended.row, 0);
        assert_eq!(before_fourth, moved_fourth);
    }
}
