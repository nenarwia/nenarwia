use std::path::{Path, PathBuf};

use crate::core::tab_session::TabSessionDocumentMode;

pub enum CanvasDocumentMode {
    Empty,
    ImportedScene { asset_root_hint: Option<PathBuf> },
}

impl CanvasDocumentMode {
    pub fn empty() -> Self {
        Self::Empty
    }

    pub fn imported(asset_root_hint: Option<PathBuf>) -> Self {
        Self::ImportedScene { asset_root_hint }
    }

    pub fn asset_root_hint(&self) -> Option<&Path> {
        match self {
            Self::Empty => None,
            Self::ImportedScene { asset_root_hint } => asset_root_hint.as_deref(),
        }
    }

    pub fn sidebar_nav_item(&self) -> Option<usize> {
        None
    }

    pub fn to_session_mode(&self) -> TabSessionDocumentMode {
        TabSessionDocumentMode::Empty
    }

    pub fn from_session_mode(_mode: &TabSessionDocumentMode) -> Self {
        Self::Empty
    }
}

#[cfg(test)]
mod tests {
    use super::CanvasDocumentMode;
    use crate::core::tab_session::TabSessionDocumentMode;

    #[test]
    fn imported_mode_serializes_to_empty_session_mode() {
        let mode = CanvasDocumentMode::imported(None);

        assert_eq!(mode.to_session_mode(), TabSessionDocumentMode::Empty);
    }
}
