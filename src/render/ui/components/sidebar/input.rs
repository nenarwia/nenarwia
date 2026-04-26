use winit::dpi::PhysicalPosition;
use winit::keyboard::ModifiersState;

use super::super::notice_texture::point_in_rect;
use super::super::{UiAction, UiClickable, CHROME_HEIGHT_PX, DEBUG_SLOT_TOGGLE_ENABLED};
use super::nav::visible_nav_items;
use super::SidebarUi;

impl SidebarUi {
    pub fn contains_screen_point(&self, pos: PhysicalPosition<f64>) -> bool {
        let x = pos.x as f32;
        let y = pos.y as f32;
        if point_in_rect(x, y, self.burger_rect_px) {
            return true;
        }
        self.open_t > 0.01 && point_in_rect(x, y, self.panel_rect_px)
    }

    pub fn handle_cursor_moved(&mut self, pos: PhysicalPosition<f64>) -> bool {
        let (x, y) = (pos.x as f32, pos.y as f32);
        let hovered_burger = point_in_rect(x, y, self.burger_rect_px);
        let mut hovered_nav_item = None;
        let mut hovered_debug_slot_backdrop = false;
        let mut hovered_fps_toggle = false;
        let mut hovered_backend_toggle = false;
        let mut hovered_wallpaper = false;
        let mut hovered_recent_wallpaper = None;

        if self.open_t > 0.01
            && point_in_rect(x, y, self.panel_rect_px)
            && y >= CHROME_HEIGHT_PX as f32
        {
            for (idx, rect) in self
                .nav_item_rects_local
                .into_iter()
                .take(visible_nav_items().len())
                .enumerate()
            {
                let world = [self.panel_rect_px[0] + rect[0], rect[1], rect[2], rect[3]];
                if point_in_rect(x, y, world) {
                    hovered_nav_item = Some(idx);
                    break;
                }
            }
            if DEBUG_SLOT_TOGGLE_ENABLED && hovered_nav_item.is_none() {
                let debug_slot_backdrop_world = [
                    self.panel_rect_px[0] + self.debug_slot_backdrop_rect_local[0],
                    self.debug_slot_backdrop_rect_local[1],
                    self.debug_slot_backdrop_rect_local[2],
                    self.debug_slot_backdrop_rect_local[3],
                ];
                hovered_debug_slot_backdrop = point_in_rect(x, y, debug_slot_backdrop_world);
            }
            if hovered_nav_item.is_none() && !hovered_debug_slot_backdrop {
                let fps_toggle_world = [
                    self.panel_rect_px[0] + self.fps_toggle_rect_local[0],
                    self.fps_toggle_rect_local[1],
                    self.fps_toggle_rect_local[2],
                    self.fps_toggle_rect_local[3],
                ];
                hovered_fps_toggle = point_in_rect(x, y, fps_toggle_world);
            }
            if hovered_nav_item.is_none() && !hovered_debug_slot_backdrop && !hovered_fps_toggle {
                let backend_toggle_world = [
                    self.panel_rect_px[0] + self.backend_toggle_rect_local[0],
                    self.backend_toggle_rect_local[1],
                    self.backend_toggle_rect_local[2],
                    self.backend_toggle_rect_local[3],
                ];
                hovered_backend_toggle = self.graphics_backend_preference.is_some()
                    && point_in_rect(x, y, backend_toggle_world);
            }
            if hovered_nav_item.is_none()
                && !hovered_debug_slot_backdrop
                && !hovered_fps_toggle
                && !hovered_backend_toggle
            {
                let wallpaper_world = [
                    self.panel_rect_px[0] + self.wallpaper_rect_local[0],
                    self.wallpaper_rect_local[1],
                    self.wallpaper_rect_local[2],
                    self.wallpaper_rect_local[3],
                ];
                hovered_wallpaper = point_in_rect(x, y, wallpaper_world);
            }
            if hovered_nav_item.is_none()
                && !hovered_debug_slot_backdrop
                && !hovered_fps_toggle
                && !hovered_wallpaper
            {
                for (idx, rect) in self.recent_wallpaper_rects_local.iter().enumerate() {
                    let world = [self.panel_rect_px[0] + rect[0], rect[1], rect[2], rect[3]];
                    if point_in_rect(x, y, world) {
                        hovered_recent_wallpaper = Some(idx);
                        break;
                    }
                }
            }
        }

        let panel_hover_changed = hovered_nav_item != self.hovered_nav_item
            || hovered_debug_slot_backdrop != self.hovered_debug_slot_backdrop
            || hovered_fps_toggle != self.hovered_fps_toggle
            || hovered_backend_toggle != self.hovered_backend_toggle
            || hovered_wallpaper != self.hovered_wallpaper
            || hovered_recent_wallpaper != self.hovered_recent_wallpaper;
        if panel_hover_changed {
            self.hovered_nav_item = hovered_nav_item;
            self.hovered_debug_slot_backdrop = hovered_debug_slot_backdrop;
            self.hovered_fps_toggle = hovered_fps_toggle;
            self.hovered_backend_toggle = hovered_backend_toggle;
            self.hovered_wallpaper = hovered_wallpaper;
            self.hovered_recent_wallpaper = hovered_recent_wallpaper;
            self.panel_texture_dirty = true;
        }

        let burger_hover_changed = hovered_burger != self.hovered_burger;
        if burger_hover_changed {
            self.hovered_burger = hovered_burger;
            self.burger_texture_dirty = true;
        }

        panel_hover_changed || burger_hover_changed
    }

    pub fn handle_click(&mut self, pos: PhysicalPosition<f64>) -> Option<UiAction> {
        let (x, y) = (pos.x as f32, pos.y as f32);
        if point_in_rect(x, y, self.burger_rect_px) {
            let _ = self.set_open(!self.target_open);
            return Some(UiAction::Consume);
        }
        if y < CHROME_HEIGHT_PX as f32 {
            return None;
        }

        if self.open_t <= 0.01 || !point_in_rect(x, y, self.panel_rect_px) {
            return None;
        }

        for (idx, rect) in self
            .nav_item_rects_local
            .into_iter()
            .take(visible_nav_items().len())
            .enumerate()
        {
            let world = [self.panel_rect_px[0] + rect[0], rect[1], rect[2], rect[3]];
            if point_in_rect(x, y, world) {
                if let Some(action) = sidebar_nav_item_action(idx) {
                    return Some(action);
                }
                let changed = self.active_nav_item != Some(idx) || self.active_wallpaper;
                self.active_nav_item = Some(idx);
                self.active_wallpaper = false;
                if changed {
                    self.panel_texture_dirty = true;
                }
                return Some(UiAction::Consume);
            }
        }

        if DEBUG_SLOT_TOGGLE_ENABLED {
            let debug_slot_backdrop_world = [
                self.panel_rect_px[0] + self.debug_slot_backdrop_rect_local[0],
                self.debug_slot_backdrop_rect_local[1],
                self.debug_slot_backdrop_rect_local[2],
                self.debug_slot_backdrop_rect_local[3],
            ];
            if point_in_rect(x, y, debug_slot_backdrop_world) {
                return Some(UiAction::ToggleDebugSlotBackdrop);
            }
        }

        let fps_toggle_world = [
            self.panel_rect_px[0] + self.fps_toggle_rect_local[0],
            self.fps_toggle_rect_local[1],
            self.fps_toggle_rect_local[2],
            self.fps_toggle_rect_local[3],
        ];
        if point_in_rect(x, y, fps_toggle_world) {
            return Some(UiAction::ToggleVsync);
        }

        let backend_toggle_world = [
            self.panel_rect_px[0] + self.backend_toggle_rect_local[0],
            self.backend_toggle_rect_local[1],
            self.backend_toggle_rect_local[2],
            self.backend_toggle_rect_local[3],
        ];
        if self.graphics_backend_preference.is_some() && point_in_rect(x, y, backend_toggle_world) {
            return Some(UiAction::ToggleGraphicsBackend);
        }

        let wallpaper_world = [
            self.panel_rect_px[0] + self.wallpaper_rect_local[0],
            self.wallpaper_rect_local[1],
            self.wallpaper_rect_local[2],
            self.wallpaper_rect_local[3],
        ];
        if point_in_rect(x, y, wallpaper_world) {
            let changed = !self.active_wallpaper || self.active_nav_item.is_some();
            self.active_nav_item = None;
            self.active_wallpaper = true;
            if changed {
                self.panel_texture_dirty = true;
            }
            return Some(UiAction::OpenWallpaperDialog);
        }

        for (idx, rect) in self.recent_wallpaper_rects_local.iter().enumerate() {
            let world = [self.panel_rect_px[0] + rect[0], rect[1], rect[2], rect[3]];
            if point_in_rect(x, y, world) {
                let changed = !self.active_wallpaper || self.active_nav_item.is_some();
                self.active_nav_item = None;
                self.active_wallpaper = true;
                if changed {
                    self.panel_texture_dirty = true;
                }
                if let Some(item) = self.recent_wallpapers.get(idx) {
                    return Some(UiAction::OpenSavedWallpaper(item.id));
                }
                return Some(UiAction::Consume);
            }
        }

        Some(UiAction::Consume)
    }
}

fn sidebar_nav_item_action(idx: usize) -> Option<UiAction> {
    visible_nav_items().get(idx).map(|item| item.action())
}

impl UiClickable for SidebarUi {
    fn on_click(
        &mut self,
        pos: PhysicalPosition<f64>,
        _modifiers: ModifiersState,
    ) -> Option<UiAction> {
        self.handle_click(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_folder_nav_item_is_action_not_persistent_tab() {
        assert_eq!(
            sidebar_nav_item_action(0),
            Some(UiAction::OpenCanvasImportDialog)
        );
    }

    #[test]
    fn sidebar_nav_items_keep_expected_actions() {
        assert_eq!(sidebar_nav_item_action(1), Some(UiAction::OpenCacheFolder));
        assert_eq!(
            sidebar_nav_item_action(2),
            Some(UiAction::ClearCurrentCanvas)
        );
        assert_eq!(sidebar_nav_item_action(3), None);
    }
}
