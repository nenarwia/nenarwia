use std::path::PathBuf;

use super::wallpaper::{WallpaperApplyRequestState, WallpaperPreviewRequestState};

pub struct PendingTrashDelete {
    pub path: PathBuf,
}

pub struct PendingEmptySlotFillDialog {
    pub tab_id: u64,
    pub slot_id: u64,
    pub rx: std::sync::mpsc::Receiver<Option<PathBuf>>,
}

#[derive(Default)]
pub struct AppBackgroundState {
    pub canvas_import_dialog_rx: Option<std::sync::mpsc::Receiver<Option<Vec<PathBuf>>>>,
    pub empty_slot_fill_dialog: Option<PendingEmptySlotFillDialog>,
    pub wallpaper_dialog_rx: Option<std::sync::mpsc::Receiver<Option<PathBuf>>>,
    pub wallpaper_preview: WallpaperPreviewRequestState,
    pub wallpaper_apply: WallpaperApplyRequestState,
    pub trash_delete_rx: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
    pub pending_trash_delete: Option<PendingTrashDelete>,
    pub vram_budget_rx: Option<std::sync::mpsc::Receiver<Option<crate::core::vram::VramInfo>>>,
}
