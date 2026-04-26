use std::fs;
use std::io;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::core::loader::disk_cache::state_root;
use crate::spatial::view::ViewState;

const TAB_SESSION_FILE: &str = "tabs_session.json";
const TAB_SESSION_VERSION: u32 = 2;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TabSessionDocumentMode {
    #[default]
    Empty,
    #[serde(other)]
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TabSessionTabState {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub media_paths: Vec<PathBuf>,
    #[serde(default)]
    pub document_mode: TabSessionDocumentMode,
    #[serde(default)]
    pub center_x: f64,
    #[serde(default)]
    pub center_y: f64,
    #[serde(default = "default_zoom")]
    pub zoom: f64,
}

impl Default for TabSessionTabState {
    fn default() -> Self {
        Self {
            title: String::new(),
            media_paths: Vec::new(),
            document_mode: TabSessionDocumentMode::Empty,
            center_x: 0.0,
            center_y: 0.0,
            zoom: default_zoom(),
        }
    }
}

impl TabSessionTabState {
    pub fn from_parts(
        title: String,
        media_paths: Vec<PathBuf>,
        document_mode: TabSessionDocumentMode,
        view: ViewState,
    ) -> Self {
        Self {
            title,
            media_paths,
            document_mode,
            center_x: finite_or_default(view.center.x, 0.0),
            center_y: finite_or_default(view.center.y, 0.0),
            zoom: positive_finite_or_default(view.zoom, default_zoom()),
        }
    }

    pub fn build_view(self, width: u32, height: u32) -> ViewState {
        let mut view = ViewState::new(width.max(1), height.max(1));
        view.center.x = finite_or_default(self.center_x, 0.0);
        view.center.y = finite_or_default(self.center_y, 0.0);
        view.zoom = positive_finite_or_default(self.zoom, default_zoom());
        view
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TabSessionState {
    #[serde(default = "default_tab_session_version")]
    pub version: u32,
    #[serde(default = "default_tab_count")]
    pub tab_count: usize,
    #[serde(default)]
    pub active_tab: usize,
    #[serde(default)]
    pub tabs: Vec<TabSessionTabState>,
}

impl Default for TabSessionState {
    fn default() -> Self {
        Self {
            version: TAB_SESSION_VERSION,
            tab_count: default_tab_count(),
            active_tab: 0,
            tabs: vec![TabSessionTabState::default()],
        }
    }
}

impl TabSessionState {
    pub fn new(tabs: Vec<TabSessionTabState>, active_tab: usize) -> Self {
        Self {
            version: TAB_SESSION_VERSION,
            tab_count: tabs.len(),
            active_tab,
            tabs,
        }
        .sanitized()
    }

    pub fn sanitized(mut self) -> Self {
        self.version = TAB_SESSION_VERSION;
        let target_count = self.tabs.len().max(self.tab_count).max(1);
        self.tabs
            .resize(target_count, TabSessionTabState::default());
        self.tab_count = self.tabs.len();
        self.active_tab = self.active_tab.min(self.tab_count.saturating_sub(1));
        self
    }
}

pub fn load_tab_session() -> Option<TabSessionState> {
    let path = tab_session_path();
    match fs::read(&path) {
        Ok(bytes) => match serde_json::from_slice::<TabSessionState>(&bytes) {
            Ok(state) => Some(state.sanitized()),
            Err(err) => {
                log::warn!("Failed to parse tab session '{}': {err:?}", path.display());
                None
            }
        },
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => {
            log::warn!("Failed to read tab session '{}': {err:?}", path.display());
            None
        }
    }
}

pub fn save_tab_session(state: TabSessionState) -> Result<()> {
    let path = tab_session_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("create tab session dir")?;
    }
    let bytes = serde_json::to_vec_pretty(&state.sanitized()).context("serialize tab session")?;
    fs::write(&path, bytes).with_context(|| format!("write tab session '{}'", path.display()))?;
    Ok(())
}

fn tab_session_path() -> PathBuf {
    state_root().join(TAB_SESSION_FILE)
}

const fn default_tab_session_version() -> u32 {
    TAB_SESSION_VERSION
}

const fn default_tab_count() -> usize {
    1
}

const fn default_zoom() -> f64 {
    1.0
}

fn finite_or_default(value: f64, default: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        default
    }
}

fn positive_finite_or_default(value: f64, default: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        default
    }
}

#[cfg(test)]
mod tests {
    use super::{TabSessionDocumentMode, TabSessionState, TabSessionTabState};

    #[test]
    fn sanitize_keeps_at_least_one_tab() {
        let state = TabSessionState::new(Vec::new(), 0);

        assert_eq!(state.tab_count, 1);
        assert_eq!(state.active_tab, 0);
        assert_eq!(state.tabs.len(), 1);
    }

    #[test]
    fn sanitize_clamps_active_index() {
        let state = TabSessionState::new(vec![TabSessionTabState::default(); 3], 99);

        assert_eq!(state.tab_count, 3);
        assert_eq!(state.active_tab, 2);
    }

    #[test]
    fn sanitize_migrates_legacy_tab_count_into_views() {
        let state = TabSessionState {
            version: 0,
            tab_count: 2,
            active_tab: 1,
            tabs: Vec::new(),
        }
        .sanitized();

        assert_eq!(state.version, 2);
        assert_eq!(state.tab_count, 2);
        assert_eq!(state.active_tab, 1);
        assert_eq!(state.tabs.len(), 2);
        assert!(state.tabs.iter().all(|tab| tab.zoom == 1.0));
        assert!(state
            .tabs
            .iter()
            .all(|tab| tab.document_mode == TabSessionDocumentMode::Empty));
    }

    #[test]
    fn unknown_document_mode_does_not_break_session_load() {
        let json = r#"{"kind":"removed_mode","extra":42}"#;
        let mode: TabSessionDocumentMode =
            serde_json::from_str(json).expect("unknown modes should fall back");

        assert_eq!(mode, TabSessionDocumentMode::Unsupported);
    }
}
