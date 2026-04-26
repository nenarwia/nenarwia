use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CanvasSlotPath {
    Live(PathBuf),
    Tombstone(PathBuf),
}

impl CanvasSlotPath {
    pub fn live(path: PathBuf) -> Self {
        Self::Live(path)
    }

    pub fn tombstone(path: PathBuf) -> Self {
        Self::Tombstone(path)
    }

    pub fn live_path(&self) -> Option<&Path> {
        match self {
            Self::Live(path) => Some(path.as_path()),
            Self::Tombstone(_) => None,
        }
    }

    pub fn remembered_path(&self) -> &Path {
        match self {
            Self::Live(path) | Self::Tombstone(path) => path.as_path(),
        }
    }

    pub fn is_tombstone(&self) -> bool {
        matches!(self, Self::Tombstone(_))
    }
}
