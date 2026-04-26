use winit::dpi::PhysicalPosition;

#[derive(Clone, Copy, Debug)]
pub(super) struct ZoomPoint {
    pub(super) world: [f64; 2],
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct InteractionState {
    drag_active: bool,
    last_cursor: Option<PhysicalPosition<f64>>,
    zoom_point: Option<ZoomPoint>,
}

impl InteractionState {
    pub(super) fn is_drag_active(&self) -> bool {
        self.drag_active
    }

    pub(super) fn last_cursor_position(&self) -> Option<PhysicalPosition<f64>> {
        self.last_cursor
    }

    pub(super) fn zoom_point(&self) -> Option<ZoomPoint> {
        self.zoom_point
    }

    pub(super) fn begin_drag(&mut self) -> bool {
        let changed = !self.drag_active;
        self.drag_active = true;
        changed
    }

    pub(super) fn end_drag(&mut self) -> bool {
        let changed = self.drag_active;
        self.drag_active = false;
        changed
    }

    pub(super) fn record_cursor(&mut self, position: PhysicalPosition<f64>) {
        self.last_cursor = Some(position);
    }

    pub(super) fn capture_zoom_point(&mut self, world: [f64; 2]) {
        self.zoom_point = Some(ZoomPoint { world });
    }

    pub(super) fn clear_zoom_point(&mut self) {
        self.zoom_point = None;
    }

    pub(super) fn reset(&mut self) {
        self.drag_active = false;
        self.last_cursor = None;
        self.zoom_point = None;
    }
}

#[cfg(test)]
mod tests {
    use super::InteractionState;
    use winit::dpi::PhysicalPosition;

    #[test]
    fn drag_lifecycle_preserves_zoom_point_and_keeps_cursor_position() {
        let mut state = InteractionState::default();
        state.record_cursor(PhysicalPosition::new(10.0, 20.0));
        state.capture_zoom_point([1.0, -1.0]);

        assert!(state.begin_drag());
        assert!(state.is_drag_active());
        assert_eq!(state.zoom_point().unwrap().world, [1.0, -1.0]);
        assert_eq!(
            state.last_cursor_position(),
            Some(PhysicalPosition::new(10.0, 20.0))
        );

        assert!(state.end_drag());
        assert!(!state.is_drag_active());
    }

    #[test]
    fn reset_clears_transient_state() {
        let mut state = InteractionState::default();
        state.record_cursor(PhysicalPosition::new(10.0, 20.0));
        state.capture_zoom_point([1.0, 2.0]);
        state.begin_drag();

        state.reset();

        assert!(!state.is_drag_active());
        assert!(state.last_cursor_position().is_none());
        assert!(state.zoom_point().is_none());
    }
}
