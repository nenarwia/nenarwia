use std::mem;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::render::cache::PageTable;
use crate::render::context::document::{CanvasDocumentMode, CanvasSlotPath, CanvasTabState};
use crate::render::context::state::RenderContext;

use super::helpers::saved_media_paths_for_persistence;

impl RenderContext {
    fn restore_active_tab_media_if_needed(&mut self) {
        if self.scene.total_count != 0 || !self.slot_paths.is_empty() || self.media_paths.is_empty()
        {
            return;
        }

        let paths = self.media_paths.clone();
        if let Err(err) = self.add_media_to_canvas_from_paths(paths.as_slice()) {
            log::warn!("Failed to restore tab media from saved paths: {}", err);
        }
    }

    pub(super) fn save_active_runtime_to_slot(&mut self, slot_index: usize) {
        let Some(slot) = self.tabs.get(slot_index) else {
            return;
        };
        let tab_id = slot.id;
        let snapshot = self.take_active_runtime_snapshot(tab_id);
        self.tabs[slot_index] = snapshot;
        self.drop_live_document_state();
    }

    pub(super) fn activate_tab_runtime(&mut self, tab_index: usize) {
        if tab_index >= self.tabs.len() {
            return;
        }

        let width = self.gpu.size.width;
        let height = self.gpu.size.height;
        let slot = &self.tabs[tab_index];
        let placeholder = CanvasTabState::placeholder(slot.id, slot.title.clone(), width, height);
        let tab = mem::replace(&mut self.tabs[tab_index], placeholder);
        self.active_tab = tab_index;
        self.load_runtime_from_tab(tab);
        self.sync_window_chrome_tabs();
        self.restore_active_tab_media_if_needed();
    }

    fn take_active_runtime_snapshot(&mut self, id: u64) -> CanvasTabState {
        let title = self.current_tab_title();
        let (empty_scene, empty_slot_paths) = crate::render::scene::Scene::from_files(Vec::new());
        let width = self.gpu.size.width.max(1);
        let height = self.gpu.size.height.max(1);
        let scene = mem::replace(&mut self.scene, empty_scene);
        let slot_paths = mem::replace(
            &mut self.slot_paths,
            empty_slot_paths
                .into_iter()
                .map(CanvasSlotPath::live)
                .collect(),
        );
        let media_paths = saved_media_paths_for_persistence(&slot_paths, &self.media_paths);
        let view = self.view().resized(width, height);

        CanvasTabState {
            id,
            title,
            scene,
            slot_paths,
            media_paths,
            view,
            document_mode: mem::replace(&mut self.document_mode, CanvasDocumentMode::empty()),
            document_revision: self.document_revision,
            auto_frame_pending: self.background.auto_frame_pending,
            canvas_import_resume_paths: self.background.canvas_import_resume_paths.clone(),
        }
    }

    fn load_runtime_from_tab(&mut self, tab: CanvasTabState) {
        self.scene = tab.scene;
        self.slot_paths = tab.slot_paths;
        self.media_paths = tab.media_paths;
        self.document_mode = tab.document_mode;
        self.document_revision = tab.document_revision.max(1);
        self.background.auto_frame_pending = tab.auto_frame_pending;
        self.background.canvas_import_resume_paths = tab.canvas_import_resume_paths;
        self.viewport.load_view(tab.view, Instant::now());
        self.sidebar_ui
            .set_active_nav_item(self.document_mode.sidebar_nav_item());

        self.reset_runtime_after_tab_load();
    }

    fn reset_runtime_after_tab_load(&mut self) {
        let awaiting_first_autoframe =
            self.background.auto_frame_pending && self.scene.total_count == 0;

        self.background.scan_rx = None;
        self.background.scan_inflight = false;
        self.background.pending_canvas_import_paths.clear();
        self.background.pending_tombstone_refresh_paths.clear();
        self.background.system_file_drop.clear();
        self.background.scan_epoch = self.background.scan_epoch.wrapping_add(1);

        self.atlas.clear();
        self.page_table = PageTable::new(self.tile_cache.total_slots);
        self.page_directory.reset_all(&self.gpu.queue);
        self.streaming_runtime.clear_preview_cache_state();

        self.bump_stream_epoch_hard();
        self.reset_scene_runtime_state();
        self.committed_view.clear_visible_membership();
        self.clear_draw_assembly_state();
        self.reset_slot_backdrop_state();
        self.hovered_id = None;
        self.selected_id = None;
        self.pending_canvas_click = None;
        self.last_empty_slot_click = None;
        self.last_media_click = None;
        self.canvas_context_menu.close();
        self.committed_view.clear_zoom_lock();
        self.streaming_runtime.reset_slot_residency();
        let view = self.view();
        let frame_count = self.frame_count;
        self.viewport_runtime_mut().reset_for_loaded_view(
            view,
            frame_count,
            awaiting_first_autoframe,
        );
        self.quality_stats = Default::default();
        self.mark_redraw_pending();

        self.resume_canvas_import_scan_after_tab_activation();
    }

    pub(super) fn drop_live_document_state(&mut self) {
        self.background.scan_rx = None;
        self.background.scan_inflight = false;
        self.background.pending_canvas_import_paths.clear();
        self.background.pending_tombstone_refresh_paths.clear();
        self.background.canvas_import_resume_paths.clear();
        self.background.system_file_drop.clear();
        self.background.scan_epoch = self.background.scan_epoch.wrapping_add(1);
    }

    fn reset_scene_runtime_state(&mut self) {
        for raw in &mut self.scene.all_items_raw {
            raw.uv_region = [0.0, 0.0, 0.0, 0.0];
            raw.params = [-1.0, -1.0, 0.0, 0.0];
            raw.params2 = [-1.0, -1.0, 0.0, 0.0];
            raw.sample_flags = [0.0, 0.0, 0.0, 0.0];
        }
        for lod in &mut self.scene.last_lod {
            *lod = 0;
        }
        for lod in &mut self.scene.display_lod {
            *lod = u8::MAX;
        }
        for lod in &mut self.scene.render_lod {
            *lod = u8::MAX;
        }
        for lod in &mut self.scene.coarse_lod {
            *lod = u8::MAX;
        }
        for debt in &mut self.scene.quality_debt {
            *debt = 0.0;
        }
    }

    pub(super) fn replace_active_with_blank(&mut self, id: u64) {
        self.drop_live_document_state();
        let blank = CanvasTabState::new_blank(id, self.gpu.size.width, self.gpu.size.height);
        self.tabs.clear();
        self.tabs.push(blank);
        self.active_tab = 0;
        self.activate_tab_runtime(0);
    }

    pub(crate) fn active_tab_id(&self) -> u64 {
        self.tabs
            .get(self.active_tab)
            .map(|tab| tab.id)
            .unwrap_or_default()
    }

    pub(crate) fn active_document_asset_root(&self) -> Option<&Path> {
        self.document_mode.asset_root_hint()
    }

    pub(crate) fn active_document_asset_root_cloned(&self) -> Option<PathBuf> {
        self.active_document_asset_root().map(Path::to_path_buf)
    }
}
