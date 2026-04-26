use std::sync::mpsc::TryRecvError;

use crate::render::context::state::RenderContext;

impl RenderContext {
    pub(super) fn poll_wallpaper_preview_result(&mut self) {
        if !self.app_background.wallpaper_preview.loading {
            return;
        }
        let Some(rx) = self.app_background.wallpaper_preview.rx.as_ref() else {
            self.app_background.wallpaper_preview.loading = false;
            return;
        };
        match rx.try_recv() {
            Ok(Ok(result)) => {
                if !self
                    .app_background
                    .wallpaper_preview
                    .is_current(result.request_epoch)
                {
                    self.app_background.wallpaper_preview.clear();
                    return;
                }
                if let Err(err) = self.wallpaper_preview_ui.open_from_rgba(
                    &self.gpu.device,
                    &self.gpu.queue,
                    self.gpu.size,
                    result.width,
                    result.height,
                    result.pixels,
                    result.blurred_pixels,
                    result.blurred_width,
                    result.blurred_height,
                    result.selected_source_path,
                    result.editing_wallpaper_id,
                    result.dim_amount,
                    result.blur_enabled,
                ) {
                    log::warn!("Failed to open wallpaper preview: {}", err);
                }
                self.app_background.wallpaper_preview.clear();
            }
            Ok(Err(err)) => {
                log::warn!("Wallpaper preview load failed: {}", err);
                self.app_background.wallpaper_preview.clear();
            }
            Err(TryRecvError::Disconnected) => {
                self.app_background.wallpaper_preview.clear();
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    pub(super) fn poll_wallpaper_apply_result(&mut self) {
        let Some(rx) = self.app_background.wallpaper_apply.rx.as_ref() else {
            return;
        };

        match rx.try_recv() {
            Ok(Ok(result)) => {
                self.app_background.wallpaper_apply.clear();
                self.wallpaper_ui.set_from_rgba(
                    &self.gpu.device,
                    &self.gpu.queue,
                    result.width,
                    result.height,
                    result.pixels.as_slice(),
                );
                self.wallpaper_ui
                    .set_dimming(&self.gpu.queue, result.dim_amount);
                self.wallpaper_ui
                    .update_layout(&self.gpu.queue, self.gpu.size);
                self.recent_wallpapers = result.recent_wallpapers;
                self.sidebar_ui
                    .set_recent_wallpapers(self.recent_wallpapers.as_slice());
                self.wallpaper_library = crate::core::wallpaper::WallpaperLibrary::load_library();
                self.mark_redraw_pending();
            }
            Ok(Err(err)) => {
                log::warn!("Wallpaper apply failed: {}", err);
                self.app_background.wallpaper_apply.clear();
                self.mark_redraw_pending();
            }
            Err(TryRecvError::Disconnected) => {
                log::warn!("Wallpaper apply worker disconnected.");
                self.app_background.wallpaper_apply.clear();
                self.mark_redraw_pending();
            }
            Err(TryRecvError::Empty) => {}
        }
    }
}
