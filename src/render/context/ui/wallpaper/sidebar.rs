use crate::core::wallpaper::{WallpaperLibrary, SIDEBAR_WALLPAPER_THUMB_MAX_DIM};
use crate::render::context::state::RenderContext;
use crate::render::ui::SidebarSavedWallpaperItem;

use super::decode::decode_wallpaper_thumbnail;

impl RenderContext {
    pub(in crate::render::context::ui) fn refresh_recent_wallpaper_sidebar_items(&mut self) {
        self.recent_wallpapers = build_recent_wallpaper_sidebar_items(&self.wallpaper_library);
        self.sidebar_ui
            .set_recent_wallpapers(self.recent_wallpapers.as_slice());
    }
}

pub(super) fn build_recent_wallpaper_sidebar_items(
    library: &WallpaperLibrary,
) -> Vec<SidebarSavedWallpaperItem> {
    let active_id = library.active_id();
    let mut recent_wallpapers = Vec::with_capacity(library.entries().len());
    for entry in library.entries() {
        match decode_wallpaper_thumbnail(&entry.source_path, SIDEBAR_WALLPAPER_THUMB_MAX_DIM) {
            Ok((pixels, width, height)) => recent_wallpapers.push(SidebarSavedWallpaperItem {
                id: entry.id,
                thumb_pixels: pixels,
                thumb_width: width,
                thumb_height: height,
                is_current: Some(entry.id) == active_id,
            }),
            Err(err) => {
                log::warn!(
                    "Failed to build sidebar thumbnail for saved wallpaper '{}': {err:#}",
                    entry.source_path.display()
                );
            }
        }
    }
    recent_wallpapers
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::core::wallpaper::WallpaperLibrary;

    #[test]
    fn sidebar_wallpaper_items_include_active_thumbnail() {
        let root = unique_temp_dir("sidebar_wallpaper_items");
        std::fs::create_dir_all(&root).expect("create temp root");
        let source = root.join("source.png");
        let image = image::RgbaImage::from_pixel(4, 3, image::Rgba([40, 80, 120, 255]));
        image.save(&source).expect("save source image");

        let mut library = WallpaperLibrary::load_from_root(root.join("wallpapers"));
        let entry = library
            .create_from_new_source(&source, false, 0.25, 320)
            .expect("create wallpaper");

        let items = super::build_recent_wallpaper_sidebar_items(&library);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, entry.id);
        assert!(items[0].is_current);
        assert!(items[0].thumb_width > 0);
        assert!(items[0].thumb_height > 0);
        assert_eq!(
            items[0].thumb_pixels.len(),
            items[0].thumb_width as usize * items[0].thumb_height as usize * 4
        );

        let _ = std::fs::remove_dir_all(root);
    }

    fn unique_temp_dir(name: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("canvas_engine_{name}_{stamp}"))
    }
}
