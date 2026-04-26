use crate::render::context::state::RenderContext;

use super::geometry::CanvasImageGeometry;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::render::context::ui) enum CanvasImageNavDirection {
    Up,
    Down,
    Left,
    Right,
}

fn notebook_after(candidate: CanvasImageGeometry, current: CanvasImageGeometry) -> bool {
    candidate.row > current.row || (candidate.row == current.row && candidate.col > current.col)
}

fn notebook_before(candidate: CanvasImageGeometry, current: CanvasImageGeometry) -> bool {
    candidate.row < current.row || (candidate.row == current.row && candidate.col < current.col)
}

fn notebook_key(geometry: CanvasImageGeometry) -> (usize, u8, u8, u64) {
    (geometry.block_pos, geometry.row, geometry.col, geometry.id)
}

fn notebook_key_in_block(geometry: CanvasImageGeometry) -> (u8, u8, u64) {
    (geometry.row, geometry.col, geometry.id)
}

fn next_notebook_geometry(
    geometries: &[CanvasImageGeometry],
    current: CanvasImageGeometry,
) -> Option<CanvasImageGeometry> {
    geometries
        .iter()
        .copied()
        .filter(|candidate| {
            candidate.block_pos == current.block_pos && notebook_after(*candidate, current)
        })
        .min_by_key(|candidate| notebook_key_in_block(*candidate))
        .or_else(|| first_geometry_after_block(geometries, current.block_pos))
}

fn previous_notebook_geometry(
    geometries: &[CanvasImageGeometry],
    current: CanvasImageGeometry,
) -> Option<CanvasImageGeometry> {
    geometries
        .iter()
        .copied()
        .filter(|candidate| {
            candidate.block_pos == current.block_pos && notebook_before(*candidate, current)
        })
        .max_by_key(|candidate| notebook_key_in_block(*candidate))
        .or_else(|| last_geometry_before_block(geometries, current.block_pos))
}

fn first_geometry_after_block(
    geometries: &[CanvasImageGeometry],
    block_pos: usize,
) -> Option<CanvasImageGeometry> {
    geometries
        .iter()
        .copied()
        .filter(|candidate| candidate.block_pos > block_pos)
        .min_by_key(|candidate| notebook_key(*candidate))
}

fn last_geometry_before_block(
    geometries: &[CanvasImageGeometry],
    block_pos: usize,
) -> Option<CanvasImageGeometry> {
    geometries
        .iter()
        .copied()
        .filter(|candidate| candidate.block_pos < block_pos)
        .max_by_key(|candidate| notebook_key(*candidate))
}

fn vertical_notebook_geometry(
    geometries: &[CanvasImageGeometry],
    current: CanvasImageGeometry,
    down: bool,
) -> Option<CanvasImageGeometry> {
    geometries
        .iter()
        .copied()
        .filter(|candidate| candidate.block_pos == current.block_pos)
        .filter(|candidate| {
            if down {
                candidate.row > current.row
            } else {
                candidate.row < current.row
            }
        })
        .min_by_key(|candidate| {
            let row_delta = if down {
                candidate.row.saturating_sub(current.row)
            } else {
                current.row.saturating_sub(candidate.row)
            };
            let col_delta = candidate.col.abs_diff(current.col);
            (row_delta, col_delta, candidate.col, candidate.id)
        })
}

pub(super) fn adjacent_canvas_image_geometry(
    geometries: &[CanvasImageGeometry],
    current: CanvasImageGeometry,
    direction: CanvasImageNavDirection,
) -> Option<CanvasImageGeometry> {
    match direction {
        CanvasImageNavDirection::Right => next_notebook_geometry(geometries, current),
        CanvasImageNavDirection::Left => previous_notebook_geometry(geometries, current),
        CanvasImageNavDirection::Down => vertical_notebook_geometry(geometries, current, true)
            .or_else(|| first_geometry_after_block(geometries, current.block_pos)),
        CanvasImageNavDirection::Up => vertical_notebook_geometry(geometries, current, false)
            .or_else(|| last_geometry_before_block(geometries, current.block_pos)),
    }
}

impl RenderContext {
    pub(in crate::render::context::ui) fn fit_adjacent_canvas_image_to_view(
        &mut self,
        direction: CanvasImageNavDirection,
    ) -> bool {
        let Some(current) = self.current_canvas_navigation_geometry() else {
            return false;
        };
        let Some(next) = self.adjacent_canvas_image_geometry(current, direction) else {
            return false;
        };

        self.last_media_click = None;
        self.fit_canvas_image_to_view(next.id)
    }
}

#[cfg(test)]
mod tests {
    use super::{next_notebook_geometry, previous_notebook_geometry, vertical_notebook_geometry};
    use crate::render::context::ui::canvas_interaction::geometry::CanvasImageGeometry;

    fn geometry(id: u64, block_pos: usize, row: u8, col: u8) -> CanvasImageGeometry {
        CanvasImageGeometry {
            id,
            center_x: col as f64,
            center_y: -(row as f64),
            bounds: (0.0, 0.0, 1.0, 1.0),
            block_pos,
            row,
            col,
        }
    }

    #[test]
    fn notebook_navigation_wraps_rows_and_blocks() {
        let items = [
            geometry(1, 0, 0, 0),
            geometry(2, 0, 0, 1),
            geometry(3, 0, 1, 0),
            geometry(4, 1, 0, 0),
        ];

        assert_eq!(
            next_notebook_geometry(&items, items[1]).map(|g| g.id),
            Some(3)
        );
        assert_eq!(
            next_notebook_geometry(&items, items[2]).map(|g| g.id),
            Some(4)
        );
        assert_eq!(
            previous_notebook_geometry(&items, items[3]).map(|g| g.id),
            Some(3)
        );
    }

    #[test]
    fn vertical_navigation_prefers_nearest_column_on_neighbor_row() {
        let items = [
            geometry(1, 0, 0, 2),
            geometry(2, 0, 1, 0),
            geometry(3, 0, 1, 2),
            geometry(4, 0, 1, 4),
        ];

        assert_eq!(
            vertical_notebook_geometry(&items, items[0], true).map(|g| g.id),
            Some(3)
        );
        assert_eq!(
            vertical_notebook_geometry(&items, items[3], false).map(|g| g.id),
            Some(1)
        );
    }
}
