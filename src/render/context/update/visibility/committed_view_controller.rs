use crate::render::context::state::RenderContext;

pub(super) fn refresh_committed_view_state(ctx: &mut RenderContext) {
    update_hovered_id(ctx);
}

fn update_hovered_id(ctx: &mut RenderContext) {
    ctx.hovered_id = ctx.cursor_pos.and_then(|pos| {
        if ctx.cursor_over_canvas_blocking_ui(pos) {
            return None;
        }
        let wp = ctx.view_metrics().screen_to_world(pos);
        super::visibility_resolver::item_id_at_world_point(ctx, wp)
    });
}
