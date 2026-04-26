use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::mpsc;

use crate::core::wallpaper::wallpaper_blur_max_dim_for_surface;
use crate::render::context::state::{RenderContext, WallpaperPreviewLoadResult};

use super::decode::{decode_saved_wallpaper_preview_blur, decode_wallpaper_preview_source};

impl RenderContext {
    pub(in crate::render::context::ui) fn open_wallpaper_preview_from_path_impl(
        &mut self,
        path: &Path,
    ) -> Result<(), String> {
        self.open_wallpaper_preview_job(path.to_path_buf(), None, 0.0, false, None)
    }

    pub(in crate::render::context::ui) fn open_saved_wallpaper_preview_impl(
        &mut self,
        id: u64,
    ) -> Result<(), String> {
        let Some(entry) = self.wallpaper_library.load_entry_for_preview(id) else {
            return Err(format!("Saved wallpaper {id} was not found."));
        };
        self.open_wallpaper_preview_job(
            entry.source_path.clone(),
            Some(entry.id),
            entry.dim_amount,
            entry.blur_enabled,
            entry
                .blur_enabled
                .then(|| self.wallpaper_library.preview_blur_path(entry.id)),
        )
    }

    pub(in crate::render::context::ui) fn wallpaper_preview_toggle_blur_impl(
        &mut self,
    ) -> Result<(), String> {
        self.wallpaper_preview_ui
            .toggle_blur(&self.gpu.device, &self.gpu.queue, self.gpu.size)
    }

    pub(in crate::render::context::ui) fn wallpaper_preview_cancel_impl(&mut self) {
        self.invalidate_wallpaper_preview_request();
        self.wallpaper_preview_ui.cancel();
    }

    pub(super) fn invalidate_wallpaper_preview_request(&mut self) {
        self.app_background.wallpaper_preview.invalidate();
    }

    fn begin_wallpaper_preview_request(&mut self) -> u64 {
        self.app_background.wallpaper_preview.begin_request()
    }

    fn open_wallpaper_preview_job(
        &mut self,
        selected_source_path: PathBuf,
        editing_wallpaper_id: Option<u64>,
        dim_amount: f32,
        blur_enabled: bool,
        saved_preview_blur_path: Option<PathBuf>,
    ) -> Result<(), String> {
        let request_epoch = self.begin_wallpaper_preview_request();
        self.wallpaper_preview_ui.open_loading(
            &self.gpu.device,
            &self.gpu.queue,
            self.gpu.size,
            Some(selected_source_path.clone()),
            editing_wallpaper_id,
            dim_amount,
        )?;

        let (tx, rx) = mpsc::channel();
        let max_dim = self
            .gpu
            .size
            .width
            .max(self.gpu.size.height)
            .clamp(960, 1920);
        let preview_blur_max_dim =
            wallpaper_blur_max_dim_for_surface(self.gpu.size.width, self.gpu.size.height);
        let preview_epoch = self.app_background.wallpaper_preview.epoch.clone();
        std::thread::spawn(move || {
            let result = decode_wallpaper_preview_source(selected_source_path.clone(), max_dim)
                .and_then(|(pixels, width, height)| {
                    if preview_epoch.load(Ordering::Relaxed) != request_epoch {
                        return Ok(None);
                    }
                    Ok(Some(WallpaperPreviewLoadResult {
                        request_epoch,
                        pixels,
                        width,
                        height,
                        blurred_pixels: None,
                        blurred_width: 0,
                        blurred_height: 0,
                        selected_source_path,
                        editing_wallpaper_id,
                        dim_amount,
                        blur_enabled,
                    }))
                })
                .and_then(|result| {
                    let Some(mut result) = result else {
                        return Ok(None);
                    };
                    if let Some(blur_path) = saved_preview_blur_path.as_ref() {
                        if preview_epoch.load(Ordering::Relaxed) != request_epoch {
                            return Ok(None);
                        }
                        let (blurred_pixels, blurred_width, blurred_height) =
                            decode_saved_wallpaper_preview_blur(
                                result.selected_source_path.as_path(),
                                blur_path.as_path(),
                                max_dim,
                                preview_blur_max_dim,
                            )?;
                        if preview_epoch.load(Ordering::Relaxed) != request_epoch {
                            return Ok(None);
                        }
                        result.blurred_pixels = Some(blurred_pixels);
                        result.blurred_width = blurred_width;
                        result.blurred_height = blurred_height;
                    }
                    Ok(Some(result))
                });
            match result {
                Ok(Some(result)) => {
                    let _ = tx.send(Ok(result));
                }
                Ok(None) => {}
                Err(err) => {
                    let _ = tx.send(Err(err));
                }
            }
        });
        self.app_background.wallpaper_preview.set_pending(rx);
        Ok(())
    }
}
