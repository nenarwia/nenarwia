use crate::core::index::stable_path_key;
use crate::render::context::document::CanvasSlotPath;
use crate::render::context::document::ScanResult;
use crate::render::context::state::RenderContext;
use crate::render::scene::SceneTailPlan;

impl RenderContext {
    pub(super) fn apply_scan_result(&mut self, result: ScanResult) {
        let ScanResult {
            restores, batch, ..
        } = result;
        let mut changed = false;

        for restore in restores {
            let Some(slot_path) = self.slot_paths.get(restore.idx) else {
                continue;
            };
            if !slot_path.is_tombstone()
                || stable_path_key(slot_path.remembered_path())
                    != stable_path_key(restore.file.path.as_path())
            {
                continue;
            }
            if self
                .scene
                .restore_item_media_slot(
                    restore.idx,
                    restore.file.asset_key,
                    (restore.file.width, restore.file.height),
                )
                .is_none()
            {
                continue;
            }
            self.slot_paths[restore.idx] = CanvasSlotPath::live(restore.file.path);
            changed = true;
        }

        if !batch.is_empty() {
            if let Some(tail) = batch.tail.as_ref() {
                match tail {
                    SceneTailPlan::Refill(tail_refill) => {
                        let old_len = self
                            .scene
                            .layout_blocks
                            .last()
                            .filter(|block| block.block_id == tail_refill.block_id)
                            .map(|block| block.index_len)
                            .unwrap_or(0);
                        self.slot_paths.extend(
                            tail_refill
                                .entries
                                .iter()
                                .skip(old_len)
                                .map(|entry| CanvasSlotPath::live(entry.file.path.clone())),
                        );
                    }
                    SceneTailPlan::Append(tail_append) => {
                        self.slot_paths.extend(
                            tail_append
                                .entries
                                .iter()
                                .map(|entry| CanvasSlotPath::live(entry.file.path.clone())),
                        );
                    }
                }
            }
            self.slot_paths
                .extend(batch.blocks.iter().flat_map(|block| {
                    block
                        .entries
                        .iter()
                        .map(|entry| CanvasSlotPath::live(entry.file.path.clone()))
                }));
            self.scene.append_batch(batch);
            changed = true;
        }

        if changed {
            self.apply_scene_append_effect();
        } else {
            self.sync_window_chrome_tabs();
        }
    }
}
