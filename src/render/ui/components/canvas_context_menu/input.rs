use winit::dpi::{PhysicalPosition, PhysicalSize};

use super::super::{notice_texture::point_in_rect, UiAction};
use super::CanvasContextMenuUi;

impl CanvasContextMenuUi {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn is_busy(&self) -> bool {
        self.busy
    }

    pub fn target_slot_id(&self) -> Option<u64> {
        self.target_slot_id
    }

    pub fn set_busy(&mut self, busy: bool) {
        if self.busy == busy {
            return;
        }
        self.busy = busy;
        self.texture_dirty = true;
    }

    pub fn open_at(
        &mut self,
        surface_size: PhysicalSize<u32>,
        cursor_pos: PhysicalPosition<f64>,
        target_slot_id: u64,
    ) {
        self.open = true;
        self.busy = false;
        self.target_slot_id = Some(target_slot_id);
        self.hovered_show_in_explorer = false;
        self.hovered_delete = false;
        self.texture_dirty = true;
        self.clamp_to_surface(surface_size, cursor_pos.x as f32, cursor_pos.y as f32);
    }

    pub fn close(&mut self) {
        self.open = false;
        self.busy = false;
        self.target_slot_id = None;
        self.hovered_show_in_explorer = false;
        self.hovered_delete = false;
        self.texture_dirty = true;
    }

    pub fn clear_target_if_matches(&mut self, slot_id: u64) {
        if self.target_slot_id == Some(slot_id) {
            self.close();
        }
    }

    pub fn contains_screen_point(&self, pos: PhysicalPosition<f64>) -> bool {
        if !self.open {
            return false;
        }
        point_in_rect(pos.x as f32, pos.y as f32, self.panel_rect_px)
    }

    pub fn handle_cursor_moved(&mut self, pos: PhysicalPosition<f64>) -> bool {
        if !self.open {
            return false;
        }

        let (x, y) = (pos.x as f32, pos.y as f32);
        let show_in_explorer_world = [
            self.panel_rect_px[0] + self.show_in_explorer_rect_local[0],
            self.panel_rect_px[1] + self.show_in_explorer_rect_local[1],
            self.show_in_explorer_rect_local[2],
            self.show_in_explorer_rect_local[3],
        ];
        let delete_world = [
            self.panel_rect_px[0] + self.delete_rect_local[0],
            self.panel_rect_px[1] + self.delete_rect_local[1],
            self.delete_rect_local[2],
            self.delete_rect_local[3],
        ];
        let hovered_show_in_explorer = !self.busy
            && point_in_rect(x, y, self.panel_rect_px)
            && point_in_rect(x, y, show_in_explorer_world);
        let hovered_delete = !self.busy
            && point_in_rect(x, y, self.panel_rect_px)
            && point_in_rect(x, y, delete_world);
        if hovered_show_in_explorer == self.hovered_show_in_explorer
            && hovered_delete == self.hovered_delete
        {
            return false;
        }

        self.hovered_show_in_explorer = hovered_show_in_explorer;
        self.hovered_delete = hovered_delete;
        self.texture_dirty = true;
        true
    }

    pub fn handle_left_click(&self, pos: PhysicalPosition<f64>) -> Option<UiAction> {
        if !self.open || self.busy {
            return None;
        }

        let target_slot_id = self.target_slot_id?;
        let x = pos.x as f32;
        let y = pos.y as f32;
        let show_in_explorer_world = [
            self.panel_rect_px[0] + self.show_in_explorer_rect_local[0],
            self.panel_rect_px[1] + self.show_in_explorer_rect_local[1],
            self.show_in_explorer_rect_local[2],
            self.show_in_explorer_rect_local[3],
        ];
        let delete_world = [
            self.panel_rect_px[0] + self.delete_rect_local[0],
            self.panel_rect_px[1] + self.delete_rect_local[1],
            self.delete_rect_local[2],
            self.delete_rect_local[3],
        ];
        if point_in_rect(x, y, show_in_explorer_world) {
            return Some(UiAction::ShowInExplorer(target_slot_id));
        }
        if point_in_rect(x, y, delete_world) {
            return Some(UiAction::MoveSlotToTrash(target_slot_id));
        }
        None
    }
}
