#[path = "wallpaper/apply.rs"]
mod apply;
#[path = "wallpaper/decode.rs"]
mod decode;
#[path = "wallpaper/preview.rs"]
mod preview;
#[path = "wallpaper/runtime.rs"]
mod runtime;
#[path = "wallpaper/sidebar.rs"]
mod sidebar;

const DEFAULT_WALLPAPER_BYTES: &[u8] = include_bytes!("../../../../assets/wall.jpeg");
