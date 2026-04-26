mod clicks;
mod geometry;
mod hit_testing;
mod navigation;

pub(in crate::render::context::ui) use navigation::CanvasImageNavDirection;

use crate::render::context::state::RenderContext;

impl RenderContext {
    pub(super) fn set_selected_id(&mut self, next: Option<u64>) -> bool {
        if self.selected_id == next {
            return false;
        }
        self.selected_id = next;
        true
    }

    pub(super) fn clear_selected_id(&mut self) -> bool {
        self.set_selected_id(None)
    }
}
