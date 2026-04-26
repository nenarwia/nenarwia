use crate::render::context::state::{RenderContext, VisibleItem};

#[inline]
pub(super) fn preview_ring_index(view: (f64, f64, f64, f64), x: f64, y: f64) -> usize {
    let (min_x, max_x, min_y, max_y) = view;
    let center_x = (min_x + max_x) * 0.5;
    let center_y = (min_y + max_y) * 0.5;
    let half_w = ((max_x - min_x) * 0.5).max(1e-6);
    let half_h = ((max_y - min_y) * 0.5).max(1e-6);

    let nx = ((x - center_x).abs() / half_w).min(1.0) as f32;
    let ny = ((y - center_y).abs() / half_h).min(1.0) as f32;
    let ring_norm = nx.max(ny);

    ((ring_norm * super::PREVIEW_RING_COUNT as f32).floor() as usize)
        .min(super::PREVIEW_RING_COUNT - 1)
}

pub(super) fn build_preview_order_strict_center(
    ctx: &RenderContext,
    items: &[VisibleItem],
) -> Vec<VisibleItem> {
    if items.len() <= 1 {
        return items.to_vec();
    }

    let view = ctx.view().viewport_rect();
    let mut rings: [Vec<VisibleItem>; super::PREVIEW_RING_COUNT] =
        std::array::from_fn(|_| Vec::new());
    for item in items.iter().copied() {
        let ring = match ctx.scene.item_world_center(item.idx) {
            Some((x, y)) => preview_ring_index(view, x, y),
            None => super::PREVIEW_RING_COUNT - 1,
        };
        rings[ring].push(item);
    }

    let mut out = Vec::with_capacity(items.len());
    for ring in rings.iter() {
        out.extend(ring.iter().copied());
    }
    out
}

pub(super) fn build_preview_order_soft_center(
    ctx: &RenderContext,
    items: &[VisibleItem],
) -> Vec<VisibleItem> {
    if items.len() <= 1 {
        return items.to_vec();
    }

    let view = ctx.view().viewport_rect();
    let mut rings: [Vec<VisibleItem>; super::PREVIEW_RING_COUNT] =
        std::array::from_fn(|_| Vec::new());
    for item in items.iter().copied() {
        let ring = match ctx.scene.item_world_center(item.idx) {
            Some((x, y)) => preview_ring_index(view, x, y),
            None => super::PREVIEW_RING_COUNT - 1,
        };
        rings[ring].push(item);
    }

    let weights = if ctx.viewport_runtime().moving_recently {
        super::PREVIEW_RING_WEIGHTS_MOVING
    } else {
        super::PREVIEW_RING_WEIGHTS_IDLE
    };

    let mut cursors = [0usize; super::PREVIEW_RING_COUNT];
    let mut out = Vec::with_capacity(items.len());
    let mut remaining = items.len();

    while remaining > 0 {
        let mut progressed = false;
        for ring_idx in 0..super::PREVIEW_RING_COUNT {
            let mut grants = weights[ring_idx];
            while grants > 0 {
                let cursor = cursors[ring_idx];
                let Some(item) = rings[ring_idx].get(cursor).copied() else {
                    break;
                };
                cursors[ring_idx] = cursor + 1;
                out.push(item);
                remaining = remaining.saturating_sub(1);
                progressed = true;
                grants -= 1;
            }
        }
        if !progressed {
            break;
        }
    }

    if out.len() < items.len() {
        for ring_idx in 0..super::PREVIEW_RING_COUNT {
            while let Some(item) = rings[ring_idx].get(cursors[ring_idx]).copied() {
                cursors[ring_idx] += 1;
                out.push(item);
            }
        }
    }

    out
}

#[inline]
pub(super) fn visible_item_ring(ctx: &RenderContext, item: VisibleItem) -> usize {
    let view = ctx.view().viewport_rect();
    match ctx.scene.item_world_center(item.idx) {
        Some((x, y)) => preview_ring_index(view, x, y),
        None => super::PREVIEW_RING_COUNT - 1,
    }
}
