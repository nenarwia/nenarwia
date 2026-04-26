mod clear_canvas;
mod layout;
mod paint;
mod text;
mod thumbs;

use crate::core::app_settings::GraphicsBackendPreference;
use crate::core::process_memory::ProcessRamUsage;
use crate::render::ui::font::load_font;
use crate::render::ui::sidebar::state::{PanelTexture, SidebarSavedWallpaperItem};
use crate::render::ui::SIDEBAR_WIDTH_PX;

pub(super) fn build_sidebar_texture(
    height: u32,
    hovered_nav_item: Option<usize>,
    hovered_debug_slot_backdrop: bool,
    hovered_fps_toggle: bool,
    hovered_backend_toggle: bool,
    hovered_wallpaper: bool,
    hovered_recent_wallpaper: Option<usize>,
    active_nav_item: Option<usize>,
    active_wallpaper: bool,
    vsync_enabled: bool,
    graphics_backend_preference: Option<GraphicsBackendPreference>,
    debug_slot_backdrop_enabled: bool,
    clear_canvas_ram_usage: Option<ProcessRamUsage>,
    recent_wallpapers: &[SidebarSavedWallpaperItem],
) -> PanelTexture {
    let width = SIDEBAR_WIDTH_PX;
    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    let layout = layout::build_panel_layout(
        height,
        graphics_backend_preference.is_some(),
        recent_wallpapers.len(),
    );

    paint::paint_panel(
        &mut pixels,
        width,
        height,
        &layout,
        hovered_nav_item,
        hovered_debug_slot_backdrop,
        hovered_fps_toggle,
        hovered_backend_toggle,
        hovered_wallpaper,
        hovered_recent_wallpaper,
        vsync_enabled,
        debug_slot_backdrop_enabled,
        graphics_backend_preference.is_some(),
        recent_wallpapers,
    );

    if let Some(font) = load_font() {
        text::draw_panel_text(
            &mut pixels,
            width,
            height,
            &layout,
            &font,
            hovered_nav_item,
            hovered_debug_slot_backdrop,
            hovered_fps_toggle,
            hovered_backend_toggle,
            hovered_wallpaper,
            hovered_recent_wallpaper,
            active_nav_item,
            active_wallpaper,
            vsync_enabled,
            graphics_backend_preference,
            debug_slot_backdrop_enabled,
            clear_canvas_ram_usage,
            recent_wallpapers,
        );
    }

    PanelTexture {
        pixels,
        width,
        height,
        nav_item_rects: layout.nav_item_rects,
        debug_slot_backdrop_rect: layout.debug_slot_backdrop_rect,
        fps_toggle_rect: layout.fps_toggle_rect,
        backend_toggle_rect: layout.backend_toggle_rect,
        wallpaper_rect: layout.wallpaper_rect,
        recent_wallpaper_rects: layout.recent_wallpaper_rects,
    }
}
