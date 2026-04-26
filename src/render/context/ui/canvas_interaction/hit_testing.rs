use winit::dpi::PhysicalPosition;

use crate::render::context::state::RenderContext;
use crate::render::context::update::visibility;

impl RenderContext {
    pub(crate) fn cursor_over_canvas_blocking_ui(&self, pos: PhysicalPosition<f64>) -> bool {
        self.window_chrome.contains_screen_point(pos)
            || self.sidebar_ui.contains_screen_point(pos)
            || self.canvas_context_menu.contains_screen_point(pos)
    }

    pub(in crate::render::context::ui) fn canvas_image_id_at_screen_point(
        &self,
        pos: PhysicalPosition<f64>,
    ) -> Option<u64> {
        if self.cursor_over_canvas_blocking_ui(pos) {
            return None;
        }

        let world = self.view_metrics().screen_to_world(pos);
        visibility::media_item_id_at_world_point(self, world)
    }

    pub(in crate::render::context::ui) fn canvas_slot_id_at_screen_point(
        &self,
        pos: PhysicalPosition<f64>,
    ) -> Option<u64> {
        if self.cursor_over_canvas_blocking_ui(pos) {
            return None;
        }

        let world = self.view_metrics().screen_to_world(pos);
        visibility::item_id_at_world_point(self, world)
    }

    pub(in crate::render::context::ui) fn empty_tombstone_slot_id_at_screen_point(
        &self,
        pos: PhysicalPosition<f64>,
    ) -> Option<u64> {
        let slot_id = self.canvas_slot_id_at_screen_point(pos)?;
        let idx = self.scene.index_for_id(slot_id)?;
        self.slot_paths
            .get(idx)
            .filter(|slot_path| slot_path.is_tombstone())
            .map(|_| slot_id)
    }
}
