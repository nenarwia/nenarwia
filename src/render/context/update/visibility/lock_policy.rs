use crate::render::context::state::RenderContext;
use crate::render::context::update::streaming::force_disable_slot_interaction_gate;

use super::{SLOT_VISIBILITY_ZOOM_LOCK_VISIBLE_THRESHOLD, SLOT_VISIBILITY_ZOOM_UNLOCK_FACTOR};

pub(super) fn slot_visibility_zoom_lock_blocks_count(ctx: &mut RenderContext) -> bool {
    let Some(locked_zoom) = ctx.committed_view.slot_visibility_zoom_lock else {
        return false;
    };
    let unlock_zoom = locked_zoom * SLOT_VISIBILITY_ZOOM_UNLOCK_FACTOR;
    if !ctx.viewport_runtime().moving_recently && ctx.view().zoom >= unlock_zoom {
        ctx.committed_view.clear_zoom_lock();
        return false;
    }
    true
}

pub(super) fn apply_slot_visibility_zoom_lock(ctx: &mut RenderContext) {
    force_disable_slot_interaction_gate(ctx);
    ctx.committed_view.clear_visible_ids();
    ctx.hovered_id = None;
    ctx.streaming_runtime.clear_preview_completion_tracking();
    ctx.quality_stats.record_visible_preview_coverage_last(0, 0);
    ctx.quality_stats
        .record_preview_phase_last(0, 0, false, false, 0, 0, 0, 0);
    ctx.quality_stats.record_visible_items_last(
        SLOT_VISIBILITY_ZOOM_LOCK_VISIBLE_THRESHOLD.saturating_add(1) as u32,
        0,
    );
}

pub(super) fn begin_zoom_lock(ctx: &mut RenderContext) {
    ctx.committed_view.slot_visibility_zoom_lock = Some(ctx.view().zoom);
    apply_slot_visibility_zoom_lock(ctx);
}
