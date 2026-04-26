use std::time::Instant;

use crate::render::context::state::RenderContext;

use super::navigation::CanvasImageNavDirection;

const CANVAS_IMAGE_FIT_PADDING: f64 = 1.0;

#[derive(Clone, Copy, Debug)]
pub(super) struct CanvasImageGeometry {
    pub id: u64,
    pub center_x: f64,
    pub center_y: f64,
    pub bounds: (f32, f32, f32, f32),
    pub block_pos: usize,
    pub row: u8,
    pub col: u8,
}

fn squared_distance(dx: f64, dy: f64) -> f64 {
    dx * dx + dy * dy
}

impl RenderContext {
    pub(super) fn fit_canvas_image_to_view(&mut self, media_id: u64) -> bool {
        let Some(geometry) = self.canvas_image_geometry_for_id(media_id) else {
            return false;
        };
        self.set_selected_id(Some(media_id));
        self.viewport.set_content_bounds(self.scene.bounds());
        self.viewport
            .fit_bounds(geometry.bounds, CANVAS_IMAGE_FIT_PADDING, Instant::now())
    }

    pub(super) fn current_canvas_navigation_geometry(&self) -> Option<CanvasImageGeometry> {
        if let Some(id) = self.selected_id {
            if let Some(geometry) = self.canvas_image_geometry_for_id(id) {
                return Some(geometry);
            }
        }

        self.nearest_canvas_image_geometry_to_world_point(
            self.view().center.x,
            self.view().center.y,
        )
    }

    pub(super) fn adjacent_canvas_image_geometry(
        &self,
        current: CanvasImageGeometry,
        direction: CanvasImageNavDirection,
    ) -> Option<CanvasImageGeometry> {
        super::navigation::adjacent_canvas_image_geometry(
            self.canvas_image_geometries().as_slice(),
            current,
            direction,
        )
    }

    fn nearest_canvas_image_geometry_to_world_point(
        &self,
        x: f64,
        y: f64,
    ) -> Option<CanvasImageGeometry> {
        self.canvas_image_geometries().into_iter().min_by(|a, b| {
            let score_a = squared_distance(a.center_x - x, a.center_y - y);
            let score_b = squared_distance(b.center_x - x, b.center_y - y);
            score_a.total_cmp(&score_b).then_with(|| a.id.cmp(&b.id))
        })
    }

    fn canvas_image_geometries(&self) -> Vec<CanvasImageGeometry> {
        self.scene
            .index_to_id
            .iter()
            .copied()
            .filter_map(|id| self.canvas_image_geometry_for_id(id))
            .collect()
    }

    fn canvas_image_geometry_for_id(&self, media_id: u64) -> Option<CanvasImageGeometry> {
        let idx = self.scene.index_for_id(media_id)?;
        self.slot_paths.get(idx)?.live_path()?;
        let address = self.scene.item_slot_address(idx)?;
        let block_pos = self.scene.block_grid_lookup.get(&address.block).copied()?;
        let (center_x, center_y, width, height) = self.scene.item_fitted_world_geometry(idx)?;
        if width <= 0.0 || height <= 0.0 {
            return None;
        }

        let half_w = width as f64 * 0.5;
        let half_h = height as f64 * 0.5;
        let min_x = (center_x - half_w) as f32;
        let max_x = (center_x + half_w) as f32;
        let min_y = (center_y - half_h) as f32;
        let max_y = (center_y + half_h) as f32;
        if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
            return None;
        }
        Some(CanvasImageGeometry {
            id: media_id,
            center_x,
            center_y,
            bounds: (min_x, min_y, max_x, max_y),
            block_pos,
            row: address.row,
            col: address.col,
        })
    }
}
