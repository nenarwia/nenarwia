#[path = "ui/canvas_interaction/mod.rs"]
mod canvas_interaction;
#[path = "ui/hotkeys.rs"]
mod hotkeys;
#[path = "ui/system_file_drop.rs"]
mod system_file_drop;
#[path = "ui/wallpaper.rs"]
mod wallpaper;
#[path = "ui/window_events.rs"]
mod window_events;

use std::path::Path;

use winit::event::WindowEvent;

use super::state::{FramePacingMode, RenderContext};
use crate::core::app_settings::{self, GraphicsBackendPreference};
use crate::core::engine::frame_pacing;
use crate::render::ui::UiAction;

impl RenderContext {
    pub(crate) fn flush_stale_system_file_drop(&mut self) {
        self.flush_stale_system_file_drop_impl();
    }

    pub fn handle_ui_hotkey(&mut self, event: &WindowEvent) -> Option<UiAction> {
        self.handle_ui_hotkey_impl(event)
    }

    pub fn frame_pacing_mode(&self) -> FramePacingMode {
        self.frame_pacing_mode
    }

    pub fn set_frame_pacing_mode(&mut self, mode: FramePacingMode) -> bool {
        frame_pacing::set_frame_pacing_mode(self, mode)
    }

    pub fn toggle_vsync(&mut self) -> bool {
        self.set_frame_pacing_mode(self.frame_pacing_mode.toggled_vsync())
    }

    pub fn toggle_graphics_backend_preference(
        &mut self,
    ) -> Result<GraphicsBackendPreference, String> {
        let Some(current) = self.graphics_backend_preference else {
            return Err("Graphics backend selection is unavailable on this platform".to_string());
        };
        let next = current.toggled();
        app_settings::save_windows_graphics_backend_preference(next)
            .map_err(|err| format!("{err:#}"))?;
        self.graphics_backend_preference = Some(next);
        Ok(next)
    }

    pub fn set_debug_slot_backdrop_enabled(&mut self, enabled: bool) -> bool {
        if self.debug_slot_backdrop_enabled == enabled {
            return false;
        }
        self.debug_slot_backdrop_enabled = enabled;
        self.mark_slot_backdrop_dirty();
        true
    }

    pub fn toggle_debug_slot_backdrop(&mut self) -> bool {
        self.set_debug_slot_backdrop_enabled(!self.debug_slot_backdrop_enabled)
    }

    pub(crate) fn initialize_saved_wallpapers(&mut self) {
        self.initialize_saved_wallpapers_impl();
    }

    pub fn open_wallpaper_preview_from_path(&mut self, path: &Path) -> Result<(), String> {
        self.open_wallpaper_preview_from_path_impl(path)
    }

    pub fn open_saved_wallpaper_preview(&mut self, id: u64) -> Result<(), String> {
        self.open_saved_wallpaper_preview_impl(id)
    }

    pub fn wallpaper_preview_toggle_blur(&mut self) -> Result<(), String> {
        self.wallpaper_preview_toggle_blur_impl()
    }

    pub fn wallpaper_preview_apply(&mut self) -> Result<(), String> {
        self.wallpaper_preview_apply_impl()
    }

    pub fn wallpaper_preview_cancel(&mut self) {
        self.wallpaper_preview_cancel_impl();
    }

    pub fn handle_ui_event(&mut self, event: &WindowEvent) -> Option<UiAction> {
        self.handle_ui_event_impl(event)
    }
}
