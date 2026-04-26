use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::render::context::state::RenderContext;
use crate::render::ui::UiAction;

use super::canvas_interaction::CanvasImageNavDirection;

fn ctrl_like_action(
    ctrl_like: bool,
    physical_key: &PhysicalKey,
    active_tab: usize,
) -> Option<UiAction> {
    if !ctrl_like {
        return None;
    }

    match physical_key {
        PhysicalKey::Code(KeyCode::KeyT) => Some(UiAction::NewTab),
        PhysicalKey::Code(KeyCode::KeyW) => Some(UiAction::CloseTab(active_tab)),
        _ => None,
    }
}

fn arrow_key_direction(physical_key: &PhysicalKey) -> Option<CanvasImageNavDirection> {
    match physical_key {
        PhysicalKey::Code(KeyCode::ArrowUp) => Some(CanvasImageNavDirection::Up),
        PhysicalKey::Code(KeyCode::ArrowDown) => Some(CanvasImageNavDirection::Down),
        PhysicalKey::Code(KeyCode::ArrowLeft) => Some(CanvasImageNavDirection::Left),
        PhysicalKey::Code(KeyCode::ArrowRight) => Some(CanvasImageNavDirection::Right),
        _ => None,
    }
}

fn plain_hotkey_action(physical_key: &PhysicalKey) -> Option<UiAction> {
    match physical_key {
        PhysicalKey::Code(KeyCode::F11) => Some(UiAction::ToggleWindowFullscreen),
        _ => None,
    }
}

fn escape_should_consume(
    menu_open: bool,
    menu_busy: bool,
    cleared_pending_click: bool,
    cleared_selected: bool,
) -> bool {
    (menu_open && !menu_busy) || cleared_pending_click || cleared_selected
}

impl RenderContext {
    pub(super) fn handle_ui_hotkey_impl(&mut self, event: &WindowEvent) -> Option<UiAction> {
        if let WindowEvent::ModifiersChanged(modifiers) = event {
            self.keyboard_modifiers = modifiers.state();
            return None;
        }

        let WindowEvent::KeyboardInput { event, .. } = event else {
            return None;
        };
        if event.state != ElementState::Pressed {
            return None;
        }

        let ctrl_like =
            self.keyboard_modifiers.control_key() || self.keyboard_modifiers.super_key();
        if let Some(action) = ctrl_like_action(ctrl_like, &event.physical_key, self.active_tab) {
            return Some(action);
        }

        if let PhysicalKey::Code(KeyCode::Escape) = event.physical_key {
            let menu_open = self.canvas_context_menu.is_open();
            let menu_busy = self.canvas_context_menu.is_busy();
            let cleared_pending_click = self.clear_pending_canvas_click();
            let cleared_selected = self.clear_selected_id();

            if menu_open && !menu_busy {
                self.canvas_context_menu.close();
            }
            if escape_should_consume(
                menu_open,
                menu_busy,
                cleared_pending_click,
                cleared_selected,
            ) {
                return Some(UiAction::Consume);
            }
        }

        if let Some(direction) = arrow_key_direction(&event.physical_key) {
            if self.fit_adjacent_canvas_image_to_view(direction) {
                return Some(UiAction::Consume);
            }
        }

        if event.repeat {
            return None;
        }

        if let Some(action) = plain_hotkey_action(&event.physical_key) {
            return Some(action);
        }

        if let PhysicalKey::Code(KeyCode::Space) = event.physical_key {
            match self.refresh_canvas_media() {
                Ok(refreshed) => {
                    if refreshed {
                        return Some(UiAction::Consume);
                    }
                }
                Err(err) => {
                    log::warn!("Failed to refresh canvas media: {}", err);
                    return Some(UiAction::Consume);
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::{
        arrow_key_direction, ctrl_like_action, escape_should_consume, plain_hotkey_action,
    };
    use crate::render::context::ui::canvas_interaction::CanvasImageNavDirection;
    use crate::render::ui::UiAction;
    use winit::keyboard::{KeyCode, PhysicalKey};

    #[test]
    fn ctrl_like_shortcuts_map_to_tab_actions() {
        assert_eq!(
            ctrl_like_action(true, &PhysicalKey::Code(KeyCode::KeyT), 3),
            Some(UiAction::NewTab)
        );
        assert_eq!(
            ctrl_like_action(true, &PhysicalKey::Code(KeyCode::KeyW), 3),
            Some(UiAction::CloseTab(3))
        );
        assert_eq!(
            ctrl_like_action(false, &PhysicalKey::Code(KeyCode::KeyT), 3),
            None
        );
    }

    #[test]
    fn f11_maps_to_window_fullscreen_toggle() {
        assert_eq!(
            plain_hotkey_action(&PhysicalKey::Code(KeyCode::F11)),
            Some(UiAction::ToggleWindowFullscreen)
        );
    }

    #[test]
    fn arrows_map_to_canvas_navigation_directions() {
        assert_eq!(
            arrow_key_direction(&PhysicalKey::Code(KeyCode::ArrowUp)),
            Some(CanvasImageNavDirection::Up)
        );
        assert_eq!(
            arrow_key_direction(&PhysicalKey::Code(KeyCode::ArrowDown)),
            Some(CanvasImageNavDirection::Down)
        );
        assert_eq!(
            arrow_key_direction(&PhysicalKey::Code(KeyCode::ArrowLeft)),
            Some(CanvasImageNavDirection::Left)
        );
        assert_eq!(
            arrow_key_direction(&PhysicalKey::Code(KeyCode::ArrowRight)),
            Some(CanvasImageNavDirection::Right)
        );
        assert_eq!(arrow_key_direction(&PhysicalKey::Code(KeyCode::KeyT)), None);
    }

    #[test]
    fn escape_consumes_only_when_it_actually_clears_ui_state() {
        assert!(escape_should_consume(true, false, false, false));
        assert!(escape_should_consume(false, false, true, false));
        assert!(escape_should_consume(false, false, false, true));
        assert!(!escape_should_consume(false, false, false, false));
        assert!(!escape_should_consume(true, true, false, false));
    }
}
