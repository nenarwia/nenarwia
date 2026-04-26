use std::path::Path;

use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};

use crate::render::context::state::RenderContext;
use crate::render::ui::{UiAction, UiClickable};

impl RenderContext {
    pub(super) fn handle_ui_event_impl(&mut self, event: &WindowEvent) -> Option<UiAction> {
        match event {
            WindowEvent::DroppedFile(path) => self.handle_dropped_file(path.as_path()),
            WindowEvent::HoveredFileCancelled => self.handle_hovered_file_cancelled(),
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                let pos = self.cursor_pos?;
                match state {
                    ElementState::Pressed => self.handle_left_mouse_press(pos),
                    ElementState::Released => self.handle_left_mouse_release(pos),
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                let pos = self.cursor_pos?;
                self.handle_right_mouse_press(pos)
            }
            WindowEvent::CursorMoved { position, .. } => self.handle_cursor_moved(*position),
            WindowEvent::MouseWheel { delta, .. } => self.handle_mouse_wheel(delta),
            _ => None,
        }
    }

    fn handle_dropped_file(&mut self, path: &Path) -> Option<UiAction> {
        self.queue_system_file_drop(path);
        Some(UiAction::Consume)
    }

    fn handle_hovered_file_cancelled(&mut self) -> Option<UiAction> {
        if self.background.system_file_drop.paths.is_empty() {
            return None;
        }

        self.finalize_system_file_drop_batch();
        Some(UiAction::Consume)
    }

    fn handle_left_mouse_press(&mut self, pos: PhysicalPosition<f64>) -> Option<UiAction> {
        self.clear_pending_canvas_click();

        if let Some(action) = self.handle_left_press_with_context_menu(pos) {
            return Some(action);
        }
        if let Some(action) = self.wallpaper_preview_ui.handle_mouse_down(
            &self.gpu.device,
            &self.gpu.queue,
            self.gpu.size,
            pos,
        ) {
            return Some(action);
        }
        if let Some(action) = self.sidebar_ui.on_click(pos, self.keyboard_modifiers) {
            return Some(action);
        }
        if let Some(action) = self.window_chrome.handle_mouse_press(pos) {
            return Some(action);
        }
        if self.codec_notice.handle_click(pos) {
            return Some(UiAction::Consume);
        }

        if let Some(slot_id) = self.empty_tombstone_slot_id_at_screen_point(pos) {
            if self.register_empty_slot_click(pos, slot_id) {
                self.clear_pending_canvas_click();
                return Some(UiAction::OpenEmptySlotFillDialog(slot_id));
            }
        } else {
            self.last_empty_slot_click = None;
        }

        self.begin_pending_canvas_click(pos, self.canvas_image_id_at_screen_point(pos));
        None
    }

    fn handle_left_press_with_context_menu(
        &mut self,
        pos: PhysicalPosition<f64>,
    ) -> Option<UiAction> {
        if !self.canvas_context_menu.is_open() {
            return None;
        }

        if let Some(action) = self.canvas_context_menu.handle_left_click(pos) {
            return Some(action);
        }
        if self.canvas_context_menu.contains_screen_point(pos) || self.canvas_context_menu.is_busy()
        {
            return Some(UiAction::Consume);
        }

        self.canvas_context_menu.close();
        Some(UiAction::Consume)
    }

    fn handle_left_mouse_release(&mut self, pos: PhysicalPosition<f64>) -> Option<UiAction> {
        if self.wallpaper_preview_ui.handle_mouse_up() {
            self.clear_pending_canvas_click();
            return Some(UiAction::Consume);
        }
        if let Some(action) = self.window_chrome.handle_mouse_release(pos) {
            self.clear_pending_canvas_click();
            return Some(action);
        }
        if self.commit_pending_canvas_click(pos) {
            self.mark_redraw_pending();
        }
        None
    }

    fn handle_right_mouse_press(&mut self, pos: PhysicalPosition<f64>) -> Option<UiAction> {
        self.clear_pending_canvas_click();
        self.last_media_click = None;

        if self.canvas_context_menu.contains_screen_point(pos) {
            return Some(UiAction::Consume);
        }
        if self.window_chrome.contains_screen_point(pos)
            || self.sidebar_ui.contains_screen_point(pos)
        {
            return self.close_context_menu_if_possible();
        }
        if let Some(slot_id) = self.canvas_image_id_at_screen_point(pos) {
            self.canvas_context_menu
                .open_at(self.gpu.size, pos, slot_id);
            self.mark_redraw_pending();
            return Some(UiAction::Consume);
        }

        self.close_context_menu_if_possible()
    }

    fn close_context_menu_if_possible(&mut self) -> Option<UiAction> {
        if !self.canvas_context_menu.is_open() {
            return None;
        }
        if self.canvas_context_menu.is_busy() {
            return Some(UiAction::Consume);
        }

        self.canvas_context_menu.close();
        Some(UiAction::Consume)
    }

    fn handle_cursor_moved(&mut self, position: PhysicalPosition<f64>) -> Option<UiAction> {
        self.update_pending_canvas_click_drag(position);
        if self.wallpaper_preview_ui.handle_cursor_moved(
            &self.gpu.device,
            &self.gpu.queue,
            self.gpu.size,
            position,
        ) {
            return Some(UiAction::Consume);
        }

        let menu_consumed = self.canvas_context_menu.handle_cursor_moved(position);
        let sidebar_consumed = self.sidebar_ui.handle_cursor_moved(position);
        let chrome_consumed = self.window_chrome.handle_cursor_moved(position);
        if menu_consumed || sidebar_consumed || chrome_consumed {
            return Some(UiAction::Consume);
        }

        None
    }

    fn handle_mouse_wheel(&mut self, _delta: &MouseScrollDelta) -> Option<UiAction> {
        None
    }
}
