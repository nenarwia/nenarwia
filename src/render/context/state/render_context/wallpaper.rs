use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::render::ui::SidebarSavedWallpaperItem;

pub struct WallpaperPreviewLoadResult {
    pub request_epoch: u64,
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub blurred_pixels: Option<Vec<u8>>,
    pub blurred_width: u32,
    pub blurred_height: u32,
    pub selected_source_path: PathBuf,
    pub editing_wallpaper_id: Option<u64>,
    pub dim_amount: f32,
    pub blur_enabled: bool,
}

pub struct WallpaperApplyResult {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub dim_amount: f32,
    pub recent_wallpapers: Vec<SidebarSavedWallpaperItem>,
}

#[derive(Default)]
pub(crate) struct WallpaperPreviewRequestState {
    pub epoch: Arc<AtomicU64>,
    pub rx: Option<std::sync::mpsc::Receiver<Result<WallpaperPreviewLoadResult, String>>>,
    pub loading: bool,
}

#[derive(Default)]
pub(crate) struct WallpaperApplyRequestState {
    pub rx: Option<std::sync::mpsc::Receiver<Result<WallpaperApplyResult, String>>>,
}

impl WallpaperApplyRequestState {
    pub fn begin(
        &mut self,
        rx: std::sync::mpsc::Receiver<Result<WallpaperApplyResult, String>>,
    ) -> Result<(), String> {
        if self.rx.is_some() {
            return Err("A wallpaper apply is already in progress.".to_string());
        }
        self.rx = Some(rx);
        Ok(())
    }

    pub fn is_pending(&self) -> bool {
        self.rx.is_some()
    }

    pub fn clear(&mut self) {
        self.rx = None;
    }
}

impl WallpaperPreviewRequestState {
    pub fn begin_request(&mut self) -> u64 {
        self.clear();
        self.epoch.fetch_add(1, Ordering::Relaxed).wrapping_add(1)
    }

    pub fn invalidate(&mut self) {
        self.epoch.fetch_add(1, Ordering::Relaxed);
        self.clear();
    }

    pub fn is_current(&self, request_epoch: u64) -> bool {
        self.epoch.load(Ordering::Relaxed) == request_epoch
    }

    pub fn clear(&mut self) {
        self.rx = None;
        self.loading = false;
    }

    pub fn set_pending(
        &mut self,
        rx: std::sync::mpsc::Receiver<Result<WallpaperPreviewLoadResult, String>>,
    ) {
        self.rx = Some(rx);
        self.loading = true;
    }
}

#[cfg(test)]
mod tests {
    use super::{WallpaperApplyRequestState, WallpaperPreviewRequestState};

    #[test]
    fn wallpaper_preview_request_begin_increments_epoch_and_clears_pending() {
        let mut state = WallpaperPreviewRequestState::default();
        let (_tx, rx) = std::sync::mpsc::channel();
        state.set_pending(rx);

        let epoch = state.begin_request();

        assert_eq!(epoch, 1);
        assert!(state.rx.is_none());
        assert!(!state.loading);
        assert!(state.is_current(epoch));
    }

    #[test]
    fn wallpaper_preview_request_invalidate_clears_pending_and_advances_epoch() {
        let mut state = WallpaperPreviewRequestState::default();
        let epoch = state.begin_request();
        let (_tx, rx) = std::sync::mpsc::channel();
        state.set_pending(rx);

        state.invalidate();

        assert!(state.rx.is_none());
        assert!(!state.loading);
        assert!(!state.is_current(epoch));
        assert!(state.is_current(epoch + 1));
    }

    #[test]
    fn wallpaper_apply_request_begin_sets_pending() {
        let mut state = WallpaperApplyRequestState::default();
        let (_tx, rx) = std::sync::mpsc::channel();

        state.begin(rx).expect("begin wallpaper apply");

        assert!(state.is_pending());
    }

    #[test]
    fn wallpaper_apply_request_begin_rejects_second_pending_job() {
        let mut state = WallpaperApplyRequestState::default();
        let (_tx1, rx1) = std::sync::mpsc::channel();
        let (_tx2, rx2) = std::sync::mpsc::channel();

        state.begin(rx1).expect("begin first wallpaper apply");

        assert!(state.begin(rx2).is_err());
        assert!(state.is_pending());
    }

    #[test]
    fn wallpaper_apply_request_clear_drops_pending() {
        let mut state = WallpaperApplyRequestState::default();
        let (_tx, rx) = std::sync::mpsc::channel();
        state.begin(rx).expect("begin wallpaper apply");

        state.clear();

        assert!(!state.is_pending());
    }
}
