use winit::window::Window;

use crate::render::context::RenderContext;

use super::event_router::window_is_effectively_minimized;

pub fn check_background_tasks(ctx: &RenderContext, window: &Window) -> bool {
    if window_is_effectively_minimized(window, None) {
        return false;
    }

    let pending = ctx.has_pending_slots_current()
        || ctx.has_pending_canvas_media_slots_current()
        || ctx.loader.has_pending_work()
        || ctx.background.scan_inflight
        || ctx.background.scan_rx.is_some()
        || ctx.app_background.canvas_import_dialog_rx.is_some()
        || ctx.app_background.trash_delete_rx.is_some()
        || ctx.app_background.wallpaper_dialog_rx.is_some()
        || ctx.app_background.wallpaper_preview.loading
        || ctx.app_background.wallpaper_apply.is_pending()
        || ctx.wallpaper_preview_ui.needs_continuous_redraw();

    let continuous_redraw = ctx.viewport.is_animating()
        || ctx.sidebar_ui.is_animating()
        || ctx.needs_camera_settle_redraw()
        || pending;
    continuous_redraw
}
