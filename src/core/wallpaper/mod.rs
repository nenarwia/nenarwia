mod assets;
mod blur;
mod fs;
mod library;
mod model;

#[cfg(test)]
mod tests;

pub use blur::{
    build_blurred_pixels, ensure_saved_wallpaper_preview_blur, wallpaper_blur_max_dim_for_surface,
};
pub use library::WallpaperLibrary;
#[allow(unused_imports)]
pub use model::WallpaperLibraryState;
pub use model::{SavedWallpaperEntry, SIDEBAR_WALLPAPER_THUMB_MAX_DIM};

const WALLPAPER_LIBRARY_VERSION: u32 = 1;
const WALLPAPER_ROOT_DIR: &str = "wallpapers";
const WALLPAPER_INDEX_FILE: &str = "index.json";
const WALLPAPER_SOURCE_FILE: &str = "source.jpg";
const WALLPAPER_PREVIEW_BLUR_FILE: &str = "preview_blur.jpg";
const WALLPAPER_STORAGE_MAX_DIM: u32 = 4096;
const WALLPAPER_JPEG_QUALITY: u8 = 100;
