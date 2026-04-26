use winit::dpi::PhysicalPosition;
use winit::keyboard::ModifiersState;

use super::super::notice_texture::point_in_rect;
use super::super::{UiAction, UiClickable};
use super::state::ChromePressTarget;
use super::WindowChromeUi;

impl WindowChromeUi {
    pub fn contains_screen_point(&self, pos: PhysicalPosition<f64>) -> bool {
        let Some(bar_rect) = self.window_rect_px else {
            return false;
        };
        point_in_rect(pos.x as f32, pos.y as f32, bar_rect)
    }

    pub fn handle_cursor_moved(&mut self, pos: PhysicalPosition<f64>) -> bool {
        let Some(bar_rect) = self.window_rect_px else {
            return false;
        };
        let (x, y) = (pos.x as f32, pos.y as f32);
        if !point_in_rect(x, y, bar_rect) {
            return self.clear_hover_state();
        }

        let hovered_close_tab_visible = self
            .tab_close_rects_px
            .iter()
            .position(|rect| rect.map(|rect| point_in_rect(x, y, rect)).unwrap_or(false));
        let hovered_close_tab = hovered_close_tab_visible
            .and_then(|visible_idx| self.tab_indices.get(visible_idx).copied());
        let hovered_add_tab = self
            .add_tab_rect_px
            .map(|rect| point_in_rect(x, y, rect))
            .unwrap_or(false);
        let hovered_tab_visible = self
            .tab_rects_px
            .iter()
            .position(|rect| point_in_rect(x, y, *rect));
        let hovered_tab = hovered_close_tab.or_else(|| {
            hovered_tab_visible.and_then(|visible_idx| self.tab_indices.get(visible_idx).copied())
        });

        self.set_hover_state(hovered_tab, hovered_close_tab, hovered_add_tab)
    }

    pub fn handle_mouse_press(&mut self, pos: PhysicalPosition<f64>) -> Option<UiAction> {
        let bar_rect = self.window_rect_px?;
        let (x, y) = (pos.x as f32, pos.y as f32);
        if !point_in_rect(x, y, bar_rect) {
            self.clear_pressed_target();
            return None;
        }
        if let Some(target) = self.press_target_at(x, y) {
            self.set_pressed_target(Some(target));
            return Some(UiAction::Consume);
        }
        self.clear_pressed_target();
        for (visible_idx, rect) in self.tab_rects_px.iter().copied().enumerate() {
            if point_in_rect(x, y, rect) {
                if let Some(&tab_index) = self.tab_indices.get(visible_idx) {
                    return Some(UiAction::SelectTab(tab_index));
                }
                return Some(UiAction::Consume);
            }
        }
        if let Some(rect) = self.drag_rect_px {
            if point_in_rect(x, y, rect) {
                return Some(UiAction::StartWindowDrag);
            }
        }
        Some(UiAction::Consume)
    }

    pub fn handle_mouse_release(&mut self, pos: PhysicalPosition<f64>) -> Option<UiAction> {
        let pressed_target = self.pressed_target?;
        self.clear_pressed_target();

        let bar_rect = self.window_rect_px?;
        let (x, y) = (pos.x as f32, pos.y as f32);
        if !point_in_rect(x, y, bar_rect) {
            return Some(UiAction::Consume);
        }

        if self.press_target_at(x, y) == Some(pressed_target) {
            return Some(Self::action_for_press_target(pressed_target));
        }

        Some(UiAction::Consume)
    }

    fn press_target_at(&self, x: f32, y: f32) -> Option<ChromePressTarget> {
        if let Some(rect) = self.close_rect_px {
            if point_in_rect(x, y, rect) {
                return Some(ChromePressTarget::CloseWindow);
            }
        }
        if let Some(rect) = self.minimize_rect_px {
            if point_in_rect(x, y, rect) {
                return Some(ChromePressTarget::MinimizeWindow);
            }
        }
        if let Some(rect) = self.maximize_rect_px {
            if point_in_rect(x, y, rect) {
                return Some(ChromePressTarget::ToggleWindowMaximize);
            }
        }
        if let Some(rect) = self.add_tab_rect_px {
            if point_in_rect(x, y, rect) {
                return Some(ChromePressTarget::NewTab);
            }
        }
        for (visible_idx, rect) in self.tab_close_rects_px.iter().enumerate() {
            if rect.map(|rect| point_in_rect(x, y, rect)).unwrap_or(false) {
                if let Some(&tab_index) = self.tab_indices.get(visible_idx) {
                    return Some(ChromePressTarget::CloseTab(tab_index));
                }
                return None;
            }
        }
        None
    }

    fn action_for_press_target(target: ChromePressTarget) -> UiAction {
        match target {
            ChromePressTarget::CloseWindow => UiAction::CloseWindow,
            ChromePressTarget::MinimizeWindow => UiAction::MinimizeWindow,
            ChromePressTarget::ToggleWindowMaximize => UiAction::ToggleWindowMaximize,
            ChromePressTarget::NewTab => UiAction::NewTab,
            ChromePressTarget::CloseTab(tab_index) => UiAction::CloseTab(tab_index),
        }
    }
}

impl UiClickable for WindowChromeUi {
    fn on_click(
        &mut self,
        pos: PhysicalPosition<f64>,
        _modifiers: ModifiersState,
    ) -> Option<UiAction> {
        self.handle_mouse_press(pos)
    }
}
