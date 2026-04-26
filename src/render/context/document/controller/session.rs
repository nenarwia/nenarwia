use crate::render::context::document::{CanvasDocumentMode, CanvasTabState};
use crate::render::context::state::RenderContext;
use crate::render::ui::ChromeTabView;

use super::helpers::saved_media_paths_for_persistence;

impl RenderContext {
    pub(crate) fn restore_tab_session_or_initialize(&mut self) {
        if !self.tabs.is_empty() {
            self.sync_window_chrome_tabs();
            return;
        }

        if let Some(session) = crate::core::tab_session::load_tab_session() {
            for persisted_tab in session.tabs.iter().cloned() {
                let id = self.allocate_tab_id();
                let mut tab =
                    CanvasTabState::new_blank(id, self.gpu.size.width, self.gpu.size.height);
                let restored_view = persisted_tab
                    .clone()
                    .build_view(self.gpu.size.width, self.gpu.size.height);
                let restored_mode =
                    CanvasDocumentMode::from_session_mode(&persisted_tab.document_mode);
                tab.title = tab_title_for_index(self.tabs.len());
                tab.media_paths = persisted_tab.media_paths;
                tab.document_mode = restored_mode;
                tab.view = restored_view;
                self.tabs.push(tab);
            }
            let active_index = session.active_tab.min(self.tabs.len().saturating_sub(1));
            self.activate_tab_runtime(active_index);
            self.persist_tab_session();
            return;
        }

        self.initialize_tabs_from_active();
        self.persist_tab_session();
    }

    pub(super) fn initialize_tabs_from_active(&mut self) {
        if !self.tabs.is_empty() {
            self.sync_window_chrome_tabs();
            return;
        }

        let id = self.allocate_tab_id();
        let title = tab_title_for_index(0);
        self.tabs.push(CanvasTabState::placeholder(
            id,
            title,
            self.gpu.size.width,
            self.gpu.size.height,
        ));
        self.active_tab = 0;
        self.sync_window_chrome_tabs();
    }

    pub(crate) fn sync_window_chrome_tabs(&mut self) {
        if self.tabs.is_empty() {
            self.window_chrome.sync_tabs(&[], 0);
            return;
        }

        let tabs: Vec<ChromeTabView> = self
            .tabs
            .iter_mut()
            .enumerate()
            .map(|(idx, tab)| {
                let title = tab_title_for_index(idx);
                if tab.title != title {
                    tab.title = title.clone();
                }
                ChromeTabView { title }
            })
            .collect();
        self.window_chrome.sync_tabs(&tabs, self.active_tab);
    }

    pub(super) fn allocate_tab_id(&mut self) -> u64 {
        let next = self.next_tab_id.max(1);
        self.next_tab_id = next.saturating_add(1);
        next
    }

    pub(crate) fn persist_tab_session(&self) {
        let tabs = if self.tabs.is_empty() {
            vec![crate::core::tab_session::TabSessionTabState::from_parts(
                tab_title_for_index(0),
                saved_media_paths_for_persistence(&self.slot_paths, &self.media_paths),
                self.document_mode.to_session_mode(),
                self.view(),
            )]
        } else {
            self.tabs
                .iter()
                .enumerate()
                .map(|(idx, tab)| {
                    let title = tab_title_for_index(idx);
                    let media_paths = if idx == self.active_tab {
                        saved_media_paths_for_persistence(&self.slot_paths, &self.media_paths)
                    } else {
                        saved_media_paths_for_persistence(&tab.slot_paths, &tab.media_paths)
                    };
                    let view = if idx == self.active_tab {
                        self.view()
                    } else {
                        tab.view
                    };
                    crate::core::tab_session::TabSessionTabState::from_parts(
                        title,
                        media_paths,
                        if idx == self.active_tab {
                            self.document_mode.to_session_mode()
                        } else {
                            tab.document_mode.to_session_mode()
                        },
                        view,
                    )
                })
                .collect()
        };
        let state = crate::core::tab_session::TabSessionState::new(tabs, self.active_tab);
        if let Err(err) = crate::core::tab_session::save_tab_session(state) {
            log::warn!("Failed to save tab session: {err:#}");
        }
    }

    pub(super) fn current_tab_title(&self) -> String {
        tab_title_for_index(self.active_tab)
    }
}

fn tab_title_for_index(index: usize) -> String {
    format!("tab {}", index.saturating_add(1))
}
