use crate::render::context::state::RenderContext;

impl RenderContext {
    pub(super) fn apply_scene_append_effect(&mut self) {
        self.mark_slot_backdrop_dirty();
        self.sync_window_chrome_tabs();
        self.mark_redraw_pending();
    }
}
