use std::collections::HashSet;
use std::path::Path;
use std::sync::mpsc::{self, TryRecvError};

use crate::core::index::stable_path_key;
use crate::core::loader::disk_cache;
use crate::render::context::document::CanvasSlotPath;
use crate::render::context::state::RenderContext;
use crate::render::scene::Scene;

#[derive(Clone, Copy, Debug)]
struct ClearedActiveSlot {
    id: u64,
    asset_key: u64,
}

impl RenderContext {
    pub(crate) fn request_slot_move_to_trash(&mut self, slot_id: u64) -> Result<(), String> {
        if self.app_background.pending_trash_delete.is_some() {
            return Err("A recycle-bin delete is already in progress.".to_string());
        }

        let idx = self
            .scene
            .index_for_id(slot_id)
            .ok_or_else(|| format!("Slot {slot_id} no longer exists."))?;
        let path = self
            .slot_paths
            .get(idx)
            .and_then(|path| path.live_path())
            .map(|path| path.to_path_buf())
            .ok_or_else(|| format!("Slot {slot_id} is already empty."))?;

        let (tx, rx) = mpsc::channel();
        let trash_path = path.clone();
        std::thread::spawn(move || {
            let result = crate::core::trash::move_path_to_trash(&trash_path);
            let _ = tx.send(result);
        });

        self.app_background.trash_delete_rx = Some(rx);
        self.app_background.pending_trash_delete =
            Some(crate::render::context::state::PendingTrashDelete { path });
        self.canvas_context_menu.set_busy(true);
        self.mark_redraw_pending();
        Ok(())
    }

    pub(crate) fn poll_trash_delete_result(&mut self) {
        let Some(rx) = self.app_background.trash_delete_rx.as_ref() else {
            return;
        };

        match rx.try_recv() {
            Ok(Ok(())) => {
                let pending = self.app_background.pending_trash_delete.take();
                self.app_background.trash_delete_rx = None;
                if let Some(pending) = pending {
                    self.apply_successful_trash_delete(&pending.path);
                }
            }
            Ok(Err(err)) => {
                let pending = self.app_background.pending_trash_delete.take();
                self.app_background.trash_delete_rx = None;
                if let Some(pending) = pending {
                    log::warn!(
                        "Failed to move '{}' to recycle bin: {}",
                        pending.path.display(),
                        err
                    );
                } else {
                    log::warn!("Failed to move deleted slot source to recycle bin: {}", err);
                }
                self.canvas_context_menu.set_busy(false);
                self.mark_redraw_pending();
            }
            Err(TryRecvError::Disconnected) => {
                let pending = self.app_background.pending_trash_delete.take();
                self.app_background.trash_delete_rx = None;
                if let Some(pending) = pending {
                    log::warn!(
                        "Recycle-bin worker disconnected while deleting '{}'.",
                        pending.path.display()
                    );
                }
                self.canvas_context_menu.set_busy(false);
                self.mark_redraw_pending();
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    fn apply_successful_trash_delete(&mut self, path: &Path) {
        let target_key = stable_path_key(path);
        remove_saved_media_paths_matching(&mut self.document.media_paths, &target_key);
        let active_cleared = {
            let scene = &mut self.document.scene;
            let slot_paths = &mut self.document.slot_paths;
            clear_slots_for_path(scene, slot_paths, &target_key)
        };

        for (idx, tab) in self.tabs.iter_mut().enumerate() {
            if idx == self.active_tab {
                continue;
            }
            remove_saved_media_paths_matching(&mut tab.media_paths, &target_key);
            clear_slots_for_path(&mut tab.scene, &mut tab.slot_paths, &target_key);
        }

        self.cleanup_deleted_active_slots(&active_cleared);
        self.canvas_context_menu.close();
        self.sync_window_chrome_tabs();
        self.mark_slot_backdrop_dirty();
        self.mark_redraw_pending();
    }

    fn cleanup_deleted_active_slots(&mut self, cleared: &[ClearedActiveSlot]) {
        if cleared.is_empty() {
            return;
        }

        let cleared_ids: HashSet<u64> = cleared.iter().map(|slot| slot.id).collect();
        let cleared_assets: HashSet<u64> = cleared
            .iter()
            .map(|slot| slot.asset_key)
            .filter(|asset_key| *asset_key != 0)
            .collect();

        for slot in cleared.iter().copied() {
            self.atlas.remove_id(&self.gpu.queue, slot.id);
        }

        for asset_key in cleared_assets.iter().copied() {
            let victims = self
                .page_directory
                .invalidate_asset(&self.gpu.queue, asset_key);
            for (lod, _region) in victims {
                self.page_table.invalidate_asset_lod(asset_key, lod);
            }
            crate::core::loader::mem_cache::remove_asset(asset_key);
            if let Err(err) = disk_cache::delete_runtime_asset(asset_key) {
                log::warn!(
                    "Failed to delete runtime disk cache for asset {}: {}",
                    asset_key,
                    err
                );
            }
        }

        self.streaming_runtime
            .remove_deleted_assets(&cleared_assets, &cleared_ids);
        let loader_canceled = self
            .loader
            .cancel_epoch_assets(self.streaming_runtime.stream_epoch, &cleared_assets);
        if loader_canceled != (0, 0, 0, 0) {
            log::debug!(
                "Canceled loader work after delete: queued={} queued_slots={} slot_subscribers={} thumb_subscribers={}",
                loader_canceled.0,
                loader_canceled.1,
                loader_canceled.2,
                loader_canceled.3
            );
        }

        self.hovered_id = self.hovered_id.filter(|id| !cleared_ids.contains(id));
        self.selected_id = self.selected_id.filter(|id| !cleared_ids.contains(id));
        self.pending_canvas_click = None;
        self.last_media_click = self
            .last_media_click
            .filter(|stamp| !cleared_ids.contains(&stamp.media_id));
        if let Some(target_slot_id) = self.canvas_context_menu.target_slot_id() {
            if cleared_ids.contains(&target_slot_id) {
                self.canvas_context_menu
                    .clear_target_if_matches(target_slot_id);
            }
        }
        self.committed_view.clear_visible_membership();
        self.committed_view.clear_zoom_lock();
        self.clear_quality_visibility_tracking();
        self.clear_draw_assembly_state();
    }
}

fn clear_slots_for_path(
    scene: &mut Scene,
    slot_paths: &mut [CanvasSlotPath],
    target_key: &str,
) -> Vec<ClearedActiveSlot> {
    let mut cleared = Vec::new();
    let limit = slot_paths.len().min(scene.all_items_raw.len());

    for idx in 0..limit {
        let remembered_path = slot_paths[idx].remembered_path();
        if stable_path_key(remembered_path) != target_key {
            continue;
        }

        if let CanvasSlotPath::Live(path) = &slot_paths[idx] {
            let tombstone_path = path.clone();
            slot_paths[idx] = CanvasSlotPath::tombstone(tombstone_path);
            if let Some((id, asset_key)) = scene.clear_item_media_slot(idx) {
                cleared.push(ClearedActiveSlot { id, asset_key });
            }
        }
    }

    cleared
}

fn remove_saved_media_paths_matching(
    media_paths: &mut Vec<std::path::PathBuf>,
    target_key: &str,
) -> bool {
    let before = media_paths.len();
    media_paths.retain(|path| stable_path_key(path) != target_key);
    media_paths.len() != before
}
