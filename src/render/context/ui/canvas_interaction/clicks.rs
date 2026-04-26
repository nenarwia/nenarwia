use std::time::{Duration, Instant};

use winit::dpi::PhysicalPosition;

use crate::render::context::state::{
    EmptySlotClickStamp, MediaItemClickStamp, PendingCanvasClick, RenderContext,
};

const CANVAS_CLICK_DRAG_THRESHOLD_PX: f64 = 5.0;
const CANVAS_DOUBLE_CLICK_MS: u64 = 500;
const CANVAS_DOUBLE_CLICK_RADIUS_PX: f64 = 8.0;

fn is_empty_slot_double_click(
    previous: Option<EmptySlotClickStamp>,
    slot_id: u64,
    pos: PhysicalPosition<f64>,
    now: Instant,
) -> bool {
    previous
        .map(|prev| {
            prev.slot_id == slot_id
                && now.duration_since(prev.at) <= Duration::from_millis(CANVAS_DOUBLE_CLICK_MS)
                && (prev.pos.x - pos.x).abs() <= CANVAS_DOUBLE_CLICK_RADIUS_PX
                && (prev.pos.y - pos.y).abs() <= CANVAS_DOUBLE_CLICK_RADIUS_PX
        })
        .unwrap_or(false)
}

fn is_media_item_double_click(
    previous: Option<MediaItemClickStamp>,
    media_id: u64,
    pos: PhysicalPosition<f64>,
    now: Instant,
) -> bool {
    previous
        .map(|prev| {
            prev.media_id == media_id
                && now.duration_since(prev.at) <= Duration::from_millis(CANVAS_DOUBLE_CLICK_MS)
                && (prev.pos.x - pos.x).abs() <= CANVAS_DOUBLE_CLICK_RADIUS_PX
                && (prev.pos.y - pos.y).abs() <= CANVAS_DOUBLE_CLICK_RADIUS_PX
        })
        .unwrap_or(false)
}

fn canvas_click_dragged(
    origin: PhysicalPosition<f64>,
    pos: PhysicalPosition<f64>,
    threshold_px: f64,
) -> bool {
    let dx = pos.x - origin.x;
    let dy = pos.y - origin.y;
    (dx * dx) + (dy * dy) >= threshold_px * threshold_px
}

impl RenderContext {
    pub(in crate::render::context::ui) fn register_empty_slot_click(
        &mut self,
        pos: PhysicalPosition<f64>,
        slot_id: u64,
    ) -> bool {
        let now = Instant::now();
        let is_double = is_empty_slot_double_click(self.last_empty_slot_click, slot_id, pos, now);

        if is_double {
            self.last_empty_slot_click = None;
            true
        } else {
            self.last_empty_slot_click = Some(EmptySlotClickStamp {
                slot_id,
                at: now,
                pos,
            });
            false
        }
    }

    pub(super) fn register_media_item_click(
        &mut self,
        pos: PhysicalPosition<f64>,
        media_id: u64,
    ) -> bool {
        let now = Instant::now();
        let is_double = is_media_item_double_click(self.last_media_click, media_id, pos, now);

        if is_double {
            self.last_media_click = None;
            true
        } else {
            self.last_media_click = Some(MediaItemClickStamp {
                media_id,
                at: now,
                pos,
            });
            false
        }
    }

    pub(in crate::render::context::ui) fn begin_pending_canvas_click(
        &mut self,
        origin: PhysicalPosition<f64>,
        candidate_id: Option<u64>,
    ) {
        self.pending_canvas_click = Some(PendingCanvasClick {
            origin,
            candidate_id,
            dragged: false,
        });
    }

    pub(in crate::render::context::ui) fn clear_pending_canvas_click(&mut self) -> bool {
        self.pending_canvas_click.take().is_some()
    }

    pub(in crate::render::context::ui) fn update_pending_canvas_click_drag(
        &mut self,
        pos: PhysicalPosition<f64>,
    ) {
        let Some(pending) = self.pending_canvas_click.as_mut() else {
            return;
        };
        if pending.dragged {
            return;
        }

        if canvas_click_dragged(pending.origin, pos, CANVAS_CLICK_DRAG_THRESHOLD_PX) {
            pending.dragged = true;
        }
    }

    pub(in crate::render::context::ui) fn commit_pending_canvas_click(
        &mut self,
        pos: PhysicalPosition<f64>,
    ) -> bool {
        let Some(pending) = self.pending_canvas_click.take() else {
            return false;
        };
        if pending.dragged {
            return false;
        }

        let release_id = self.canvas_image_id_at_screen_point(pos);
        if release_id != pending.candidate_id {
            return false;
        }

        let Some(media_id) = release_id else {
            self.last_media_click = None;
            return self.clear_selected_id();
        };

        if !self.register_media_item_click(pos, media_id) {
            return false;
        }

        self.fit_canvas_image_to_view(media_id)
    }
}

#[cfg(test)]
mod tests {
    use super::{canvas_click_dragged, is_empty_slot_double_click, is_media_item_double_click};
    use crate::render::context::state::{EmptySlotClickStamp, MediaItemClickStamp};
    use std::time::{Duration, Instant};
    use winit::dpi::PhysicalPosition;

    #[test]
    fn empty_slot_double_click_requires_same_slot_time_window_and_radius() {
        let now = Instant::now();
        let previous = Some(EmptySlotClickStamp {
            slot_id: 7,
            at: now,
            pos: PhysicalPosition::new(100.0, 200.0),
        });

        assert!(is_empty_slot_double_click(
            previous,
            7,
            PhysicalPosition::new(104.0, 205.0),
            now + Duration::from_millis(200),
        ));
        assert!(!is_empty_slot_double_click(
            previous,
            8,
            PhysicalPosition::new(104.0, 205.0),
            now + Duration::from_millis(200),
        ));
        assert!(!is_empty_slot_double_click(
            previous,
            7,
            PhysicalPosition::new(120.0, 205.0),
            now + Duration::from_millis(200),
        ));
        assert!(!is_empty_slot_double_click(
            previous,
            7,
            PhysicalPosition::new(104.0, 205.0),
            now + Duration::from_millis(600),
        ));
    }

    #[test]
    fn media_item_double_click_requires_same_item_time_window_and_radius() {
        let now = Instant::now();
        let previous = Some(MediaItemClickStamp {
            media_id: 42,
            at: now,
            pos: PhysicalPosition::new(100.0, 200.0),
        });

        assert!(is_media_item_double_click(
            previous,
            42,
            PhysicalPosition::new(104.0, 205.0),
            now + Duration::from_millis(200),
        ));
        assert!(!is_media_item_double_click(
            previous,
            99,
            PhysicalPosition::new(104.0, 205.0),
            now + Duration::from_millis(200),
        ));
        assert!(!is_media_item_double_click(
            previous,
            42,
            PhysicalPosition::new(120.0, 205.0),
            now + Duration::from_millis(200),
        ));
        assert!(!is_media_item_double_click(
            previous,
            42,
            PhysicalPosition::new(104.0, 205.0),
            now + Duration::from_millis(600),
        ));
    }

    #[test]
    fn canvas_click_drag_threshold_requires_real_movement() {
        let origin = PhysicalPosition::new(10.0, 10.0);
        assert!(!canvas_click_dragged(
            origin,
            PhysicalPosition::new(13.0, 13.0),
            5.0
        ));
        assert!(canvas_click_dragged(
            origin,
            PhysicalPosition::new(15.0, 10.0),
            5.0
        ));
    }
}
