mod burger;
mod constants;
mod panel;
mod primitives;
mod text_utils;

use super::state::{BurgerTexture, PanelTexture, SidebarSavedWallpaperItem};
use crate::core::app_settings::GraphicsBackendPreference;
use crate::core::process_memory::ProcessRamUsage;

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
    panel::build_sidebar_texture(
        height,
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
    )
}

pub(super) fn build_burger_texture(hovered: bool) -> BurgerTexture {
    burger::build_burger_texture(hovered)
}

pub(super) fn write_rect_vertices(
    queue: &wgpu::Queue,
    vertex_buffer: &wgpu::Buffer,
    surface_size: winit::dpi::PhysicalSize<u32>,
    rect: [f32; 4],
) {
    primitives::write_rect_vertices(queue, vertex_buffer, surface_size, rect);
}

pub(super) fn ease_in_out_cubic(t: f32) -> f32 {
    primitives::ease_in_out_cubic(t)
}
