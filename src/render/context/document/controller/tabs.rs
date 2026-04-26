use std::path::PathBuf;

use crate::render::context::document::{CanvasDocumentMode, CanvasTabState};
use crate::render::context::state::RenderContext;

use super::helpers::cleared_document_mode_for_canvas_reset;

impl RenderContext {
    pub(crate) fn add_blank_tab(&mut self) -> bool {
        self.initialize_tabs_from_active();
        self.save_active_runtime_to_slot(self.active_tab);

        let id = self.allocate_tab_id();
        let tab = CanvasTabState::new_blank(id, self.gpu.size.width, self.gpu.size.height);
        self.tabs.push(tab);
        let new_index = self.tabs.len().saturating_sub(1);
        self.activate_tab_runtime(new_index);
        self.persist_tab_session();
        true
    }

    pub(crate) fn select_tab(&mut self, tab_index: usize) -> bool {
        self.initialize_tabs_from_active();
        if tab_index >= self.tabs.len() || tab_index == self.active_tab {
            return false;
        }

        self.save_active_runtime_to_slot(self.active_tab);
        self.activate_tab_runtime(tab_index);
        self.persist_tab_session();
        true
    }

    pub(crate) fn close_tab(&mut self, tab_index: usize) -> bool {
        self.initialize_tabs_from_active();
        if tab_index >= self.tabs.len() {
            return false;
        }

        if self.tabs.len() == 1 {
            let id = self.tabs[0].id;
            self.replace_active_with_blank(id);
            self.persist_tab_session();
            return true;
        }

        if tab_index == self.active_tab {
            self.drop_live_document_state();
            self.tabs.remove(tab_index);
            let next_index = tab_index.min(self.tabs.len().saturating_sub(1));
            self.active_tab = next_index;
            self.activate_tab_runtime(next_index);
            self.persist_tab_session();
            return true;
        }

        self.tabs.remove(tab_index);
        if self.active_tab > tab_index {
            self.active_tab = self.active_tab.saturating_sub(1);
        }
        self.sync_window_chrome_tabs();
        self.persist_tab_session();
        true
    }

    pub(crate) fn promote_active_document_to_imported(&mut self, asset_root_hint: Option<PathBuf>) {
        self.drop_live_document_state();
        self.document_mode = CanvasDocumentMode::imported(asset_root_hint);
        self.document_revision = self.document_revision.wrapping_add(1).max(1);
        self.sidebar_ui
            .set_active_nav_item(self.document_mode.sidebar_nav_item());
        self.sync_window_chrome_tabs();
    }

    pub(crate) fn clear_active_canvas(&mut self) -> bool {
        self.initialize_tabs_from_active();
        if self.active_tab >= self.tabs.len() {
            return false;
        }

        let active_idx = self.active_tab;
        let id = self.tabs[active_idx].id;
        let cleared_mode = cleared_document_mode_for_canvas_reset(&self.document_mode);
        self.save_active_runtime_to_slot(active_idx);
        self.tabs[active_idx] = CanvasTabState::new_empty_with_mode(
            id,
            self.gpu.size.width,
            self.gpu.size.height,
            cleared_mode,
        );
        self.activate_tab_runtime(active_idx);
        self.persist_tab_session();
        true
    }
}
