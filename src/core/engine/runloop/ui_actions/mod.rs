mod dialogs;
mod dispatch;
mod external;
mod titlebar;
mod window;

use winit::event_loop::EventLoopWindowTarget;
use winit::window::Window;

use crate::render::context::RenderContext;
use crate::render::ui::UiAction;

pub(super) fn apply_ui_action(
    action: UiAction,
    window: &Window,
    ctx: &mut RenderContext,
    elwt: &EventLoopWindowTarget<()>,
) {
    dispatch::apply_ui_action(action, window, ctx, elwt);
}

pub(super) fn maybe_start_pending_titlebar_drag(window: &Window, ctx: &mut RenderContext) {
    titlebar::maybe_start_pending_titlebar_drag(window, ctx);
}

fn begin_titlebar_window_drag(window: &Window, ctx: &mut RenderContext) {
    titlebar::begin_titlebar_window_drag(window, ctx);
}

fn toggle_window_maximize(window: &Window, ctx: &mut RenderContext) {
    window::toggle_window_maximize(window, ctx);
}

fn toggle_window_fullscreen(window: &Window, ctx: &mut RenderContext) {
    window::toggle_window_fullscreen(window, ctx);
}

fn close_window_and_exit(ctx: &mut RenderContext, elwt: &EventLoopWindowTarget<()>) {
    window::close_window_and_exit(ctx, elwt);
}

fn open_canvas_import_dialog(window: &Window, ctx: &mut RenderContext) {
    dialogs::open_canvas_import_dialog(window, ctx);
}

fn open_empty_slot_fill_dialog(window: &Window, ctx: &mut RenderContext, slot_id: u64) {
    dialogs::open_empty_slot_fill_dialog(window, ctx, slot_id);
}

fn open_wallpaper_dialog(window: &Window, ctx: &mut RenderContext) {
    dialogs::open_wallpaper_dialog(window, ctx);
}

fn reveal_slot_in_explorer(slot_id: u64, ctx: &mut RenderContext) {
    external::reveal_slot_in_explorer(slot_id, ctx);
}

fn open_cache_folder_in_explorer() {
    external::open_cache_folder_in_explorer();
}
