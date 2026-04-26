use std::path::PathBuf;
use std::sync::mpsc;

use crate::core::wallpaper::{wallpaper_blur_max_dim_for_surface, WallpaperLibrary};
use crate::render::context::state::{RenderContext, WallpaperApplyResult};

use super::sidebar::build_recent_wallpaper_sidebar_items;

struct WallpaperApplyRequest {
    source_path: PathBuf,
    editing_wallpaper_id: Option<u64>,
    blur_enabled: bool,
    dim_amount: f32,
    preview_blur_max_dim: u32,
    surface_width: u32,
    surface_height: u32,
}

impl RenderContext {
    pub(in crate::render::context::ui) fn wallpaper_preview_apply_impl(
        &mut self,
    ) -> Result<(), String> {
        if self.app_background.wallpaper_apply.is_pending() {
            return Err("A wallpaper apply is already in progress.".to_string());
        }

        let source_path = self
            .wallpaper_preview_ui
            .selected_source_path()
            .ok_or_else(|| "Wallpaper source is missing.".to_string())?
            .to_path_buf();
        let blur_enabled = self.wallpaper_preview_ui.blur_enabled();
        let dim_amount = self.wallpaper_preview_ui.dim_amount();
        let editing_wallpaper_id = self.wallpaper_preview_ui.editing_wallpaper_id();
        let preview_blur_max_dim =
            wallpaper_blur_max_dim_for_surface(self.gpu.size.width, self.gpu.size.height);
        let request = WallpaperApplyRequest {
            source_path,
            editing_wallpaper_id,
            blur_enabled,
            dim_amount,
            preview_blur_max_dim,
            surface_width: self.gpu.size.width,
            surface_height: self.gpu.size.height,
        };
        let (tx, rx) = mpsc::channel();
        self.app_background.wallpaper_apply.begin(rx)?;
        self.wallpaper_preview_ui.cancel();
        self.invalidate_wallpaper_preview_request();
        std::thread::spawn(move || {
            let result = apply_wallpaper_request(request);
            let _ = tx.send(result);
        });
        self.mark_redraw_pending();
        Ok(())
    }
}

fn apply_wallpaper_request(request: WallpaperApplyRequest) -> Result<WallpaperApplyResult, String> {
    let mut library = WallpaperLibrary::load_library();
    let entry = if let Some(id) = request.editing_wallpaper_id {
        library
            .update_existing(
                id,
                request.blur_enabled,
                request.dim_amount,
                request.preview_blur_max_dim,
            )
            .map_err(|err| format!("Failed to update saved wallpaper: {err:#}"))?
    } else {
        library
            .create_from_new_source(
                &request.source_path,
                request.blur_enabled,
                request.dim_amount,
                request.preview_blur_max_dim,
            )
            .map_err(|err| format!("Failed to save wallpaper: {err:#}"))?
    };

    let decoded = crate::core::color::decode_rgba8_srgb(&entry.source_path).map_err(|err| {
        format!(
            "Failed to decode saved wallpaper '{}': {err:#}",
            entry.source_path.display()
        )
    })?;
    let (pixels, width, height) = prepare_runtime_wallpaper_pixels(
        decoded.rgba.as_raw(),
        decoded.width,
        decoded.height,
        entry.blur_enabled,
        request.surface_width,
        request.surface_height,
    )?;
    let recent_wallpapers = build_recent_wallpaper_sidebar_items(&library);

    Ok(WallpaperApplyResult {
        pixels,
        width,
        height,
        dim_amount: entry.dim_amount,
        recent_wallpapers,
    })
}

pub(super) fn prepare_runtime_wallpaper_pixels(
    pixels: &[u8],
    width: u32,
    height: u32,
    blur_enabled: bool,
    surface_width: u32,
    surface_height: u32,
) -> Result<(Vec<u8>, u32, u32), String> {
    if !blur_enabled {
        return Ok((pixels.to_vec(), width, height));
    }

    let blur_max_dim = wallpaper_blur_max_dim_for_surface(surface_width, surface_height);
    crate::core::wallpaper::build_blurred_pixels(width, height, pixels, blur_max_dim)
        .map(|(blurred_pixels, blurred_width, blurred_height)| {
            (blurred_pixels, blurred_width.max(1), blurred_height.max(1))
        })
        .map_err(|err| format!("Failed to build wallpaper blur: {err:#}"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn runtime_wallpaper_pixels_use_source_when_blur_is_disabled() {
        let pixels = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
        ];

        let (runtime_pixels, width, height) =
            super::prepare_runtime_wallpaper_pixels(&pixels, 2, 2, false, 1280, 720)
                .expect("prepare runtime wallpaper");

        assert_eq!(width, 2);
        assert_eq!(height, 2);
        assert_eq!(runtime_pixels, pixels);
    }

    #[test]
    fn runtime_wallpaper_pixels_build_blur_when_enabled() {
        let pixels = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
        ];

        let (runtime_pixels, width, height) =
            super::prepare_runtime_wallpaper_pixels(&pixels, 2, 2, true, 1280, 720)
                .expect("prepare blurred runtime wallpaper");

        assert_eq!(width, 2);
        assert_eq!(height, 2);
        assert_eq!(runtime_pixels.len(), 2 * 2 * 4);
    }
}
