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
    match action {
        UiAction::Consume => {}
        UiAction::NewTab => {
            ctx.add_blank_tab();
        }
        UiAction::SelectTab(tab_index) => {
            ctx.select_tab(tab_index);
        }
        UiAction::CloseTab(tab_index) => {
            ctx.close_tab(tab_index);
        }
        UiAction::StartWindowDrag => {
            super::begin_titlebar_window_drag(window, ctx);
        }
        UiAction::ToggleWindowMaximize => {
            super::toggle_window_maximize(window, ctx);
        }
        UiAction::ToggleWindowFullscreen => {
            super::toggle_window_fullscreen(window, ctx);
        }
        UiAction::MinimizeWindow => {
            window.set_minimized(true);
        }
        UiAction::CloseWindow => {
            super::close_window_and_exit(ctx, elwt);
        }
        UiAction::ToggleVsync => {
            if ctx.toggle_vsync() {
                log::info!("Frame pacing mode: {}", ctx.frame_pacing_mode().label());
            }
        }
        UiAction::ToggleGraphicsBackend => match ctx.toggle_graphics_backend_preference() {
            Ok(preference) => {
                log::info!(
                    "Graphics backend preference saved: {}. Restart the app to apply it.",
                    preference.label()
                );
            }
            Err(err) => {
                log::warn!("Failed to toggle graphics backend preference: {}", err);
            }
        },
        UiAction::ToggleDebugSlotBackdrop => {
            if ctx.toggle_debug_slot_backdrop() {
                log::info!(
                    "Slot backdrop mode: {}",
                    if ctx.debug_slot_backdrop_enabled {
                        "debug slots"
                    } else {
                        "single frame"
                    }
                );
            }
        }
        UiAction::OpenCanvasImportDialog => {
            super::open_canvas_import_dialog(window, ctx);
        }
        UiAction::OpenCacheFolder => {
            super::open_cache_folder_in_explorer();
        }
        UiAction::ClearCurrentCanvas => {
            ctx.clear_active_canvas();
        }
        UiAction::OpenEmptySlotFillDialog(slot_id) => {
            super::open_empty_slot_fill_dialog(window, ctx, slot_id);
        }
        UiAction::MoveSlotToTrash(slot_id) => {
            if let Err(err) = ctx.request_slot_move_to_trash(slot_id) {
                log::warn!("Failed to start recycle-bin delete: {}", err);
                ctx.canvas_context_menu.set_busy(false);
            }
        }
        UiAction::ShowInExplorer(slot_id) => {
            super::reveal_slot_in_explorer(slot_id, ctx);
        }
        UiAction::OpenWallpaperDialog => {
            super::open_wallpaper_dialog(window, ctx);
        }
        UiAction::OpenSavedWallpaper(id) => {
            if let Err(err) = ctx.open_saved_wallpaper_preview(id) {
                log::warn!("Failed to open saved wallpaper {id} in preview: {err}");
            }
        }
        UiAction::WallpaperPreviewToggleBlur => {
            if let Err(err) = ctx.wallpaper_preview_toggle_blur() {
                log::warn!("Failed to toggle wallpaper blur preview: {}", err);
            }
        }
        UiAction::WallpaperPreviewApply => {
            if let Err(err) = ctx.wallpaper_preview_apply() {
                log::warn!("Failed to apply wallpaper from preview: {}", err);
            }
        }
        UiAction::WallpaperPreviewCancel => {
            ctx.wallpaper_preview_cancel();
        }
    }
}
