use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::WALLPAPER_LIBRARY_VERSION;

pub const MAX_SAVED_WALLPAPERS: usize = 10;
pub const SIDEBAR_WALLPAPER_THUMB_MAX_DIM: u32 = 192;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedWallpaperEntry {
    pub id: u64,
    pub source_path: PathBuf,
    #[serde(default)]
    pub source_hash: String,
    #[serde(default)]
    pub is_default: bool,
    pub blur_enabled: bool,
    pub dim_amount: f32,
    pub created_at: u64,
    pub updated_at: u64,
    pub last_used_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WallpaperLibraryState {
    pub version: u32,
    pub next_id: u64,
    pub active_id: Option<u64>,
    pub items: Vec<SavedWallpaperEntry>,
}

impl Default for WallpaperLibraryState {
    fn default() -> Self {
        Self {
            version: WALLPAPER_LIBRARY_VERSION,
            next_id: 1,
            active_id: None,
            items: Vec::new(),
        }
    }
}
