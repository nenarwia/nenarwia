use crate::core::wallpaper::SavedWallpaperEntry;
use crate::render::context::state::RenderContext;

use super::apply::prepare_runtime_wallpaper_pixels;
use super::decode::decode_default_wallpaper_source;

impl RenderContext {
    pub(in crate::render::context::ui) fn initialize_saved_wallpapers_impl(&mut self) {
        if let Err(err) = self
            .wallpaper_library
            .ensure_default_wallpaper(super::DEFAULT_WALLPAPER_BYTES)
        {
            log::warn!("Failed to seed bundled default wallpaper: {err:#}");
        }

        let active_entry = match self.wallpaper_library.load_active_wallpaper() {
            Ok(entry) => entry,
            Err(err) => {
                log::warn!("Failed to load saved wallpaper library: {err:#}");
                None
            }
        };
        let mut active_wallpaper_restored = false;
        if let Some(entry) = active_entry {
            match self.apply_saved_wallpaper_entry(&entry) {
                Ok(()) => {
                    active_wallpaper_restored = true;
                }
                Err(err) => {
                    log::warn!("Failed to restore active wallpaper: {err}");
                    if let Err(clear_err) = self.wallpaper_library.clear_active() {
                        log::warn!("Failed to clear broken active wallpaper: {clear_err:#}");
                    }
                }
            }
        }
        if !active_wallpaper_restored {
            if let Err(err) = self.apply_default_wallpaper() {
                log::warn!("Failed to apply bundled default wallpaper: {err}");
            }
        }
        self.refresh_recent_wallpaper_sidebar_items();
    }

    fn apply_saved_wallpaper_entry(&mut self, entry: &SavedWallpaperEntry) -> Result<(), String> {
        let decoded = crate::core::color::decode_rgba8_srgb(&entry.source_path).map_err(|err| {
            format!(
                "Failed to decode saved wallpaper '{}': {err:#}",
                entry.source_path.display()
            )
        })?;
        self.apply_wallpaper_pixels(
            decoded.rgba.as_raw(),
            decoded.width,
            decoded.height,
            entry.blur_enabled,
            entry.dim_amount,
        )
    }

    fn apply_default_wallpaper(&mut self) -> Result<(), String> {
        let decoded = decode_default_wallpaper_source()?;
        self.apply_wallpaper_pixels(
            decoded.rgba.as_raw(),
            decoded.width,
            decoded.height,
            false,
            0.0,
        )
    }

    fn apply_wallpaper_pixels(
        &mut self,
        pixels: &[u8],
        width: u32,
        height: u32,
        blur_enabled: bool,
        dim_amount: f32,
    ) -> Result<(), String> {
        let (runtime_pixels, runtime_width, runtime_height) = prepare_runtime_wallpaper_pixels(
            pixels,
            width,
            height,
            blur_enabled,
            self.gpu.size.width,
            self.gpu.size.height,
        )?;
        self.wallpaper_ui.set_from_rgba(
            &self.gpu.device,
            &self.gpu.queue,
            runtime_width,
            runtime_height,
            runtime_pixels.as_slice(),
        );
        self.wallpaper_ui.set_dimming(&self.gpu.queue, dim_amount);
        self.wallpaper_ui
            .update_layout(&self.gpu.queue, self.gpu.size);
        Ok(())
    }
}
