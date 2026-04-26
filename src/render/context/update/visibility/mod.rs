mod committed_view_controller;
mod draw_assembly;
mod lock_policy;
mod visibility_resolver;

use crate::render::context::state::RenderContext;
use crate::render::instance::InstanceRaw;

const SLOT_VISIBILITY_ZOOM_LOCK_VISIBLE_THRESHOLD: usize = 18_000;
const SLOT_VISIBILITY_ZOOM_UNLOCK_FACTOR: f64 = 1.15;

pub fn process_visible(ctx: &mut RenderContext) {
    if lock_policy::slot_visibility_zoom_lock_blocks_count(ctx) {
        lock_policy::apply_slot_visibility_zoom_lock(ctx);
        return;
    }

    let probe = visibility_resolver::probe_visible_items_with_limit(
        ctx,
        SLOT_VISIBILITY_ZOOM_LOCK_VISIBLE_THRESHOLD.saturating_add(1),
    );
    if probe.reached_limit {
        // Under zoom-lock, show every slot that already has drawable media rather than
        // freezing a viewport-clipped exact-visible set.
        ctx.committed_view.visible_items = visibility_resolver::collect_manifested_media_items(ctx);
        lock_policy::begin_zoom_lock(ctx);
        return;
    }
    ctx.committed_view.visible_items = probe.items;

    committed_view_controller::refresh_committed_view_state(ctx);
}

pub fn update_visible_buffer(ctx: &mut RenderContext) {
    draw_assembly::update_visible_buffer(ctx);
}

pub fn update_slot_backdrop_buffer(ctx: &mut RenderContext) {
    draw_assembly::update_slot_backdrop_buffer(ctx);
}

pub(crate) fn media_item_id_at_world_point(ctx: &RenderContext, point: [f64; 2]) -> Option<u64> {
    visibility_resolver::media_item_id_at_world_point(ctx, point)
}

pub(crate) fn item_id_at_world_point(ctx: &RenderContext, point: [f64; 2]) -> Option<u64> {
    visibility_resolver::item_id_at_world_point(ctx, point)
}

pub(super) fn slot_has_media_content(inst: InstanceRaw) -> bool {
    let has_preview = inst.uv_region[2] > 0.0 && inst.uv_region[3] > 0.0;
    let has_tiles = inst.params[0] >= 0.0 && inst.params[2] > 0.0 && inst.params[3] > 0.0;
    has_preview || has_tiles
}
