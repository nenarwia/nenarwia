use winit::window::Window;

use crate::core::engine::window::set_native_cursor_custom;
use crate::core::formats::FILE_DIALOG_IMAGE_EXTS;
use crate::render::context::state::PendingEmptySlotFillDialog;
use crate::render::context::RenderContext;

const WALLPAPER_DIALOG_IMAGE_EXTS: &[&str] =
    &["png", "jpg", "jpeg", "webp", "bmp", "gif", "tif", "tiff"];

pub(super) fn open_canvas_import_dialog(window: &Window, ctx: &mut RenderContext) {
    if native_file_dialog_inflight(ctx) {
        return;
    }

    set_native_cursor_custom(window);
    let (tx, rx) = std::sync::mpsc::channel();
    ctx.app_background.canvas_import_dialog_rx = Some(rx);
    std::thread::spawn(move || {
        let picked = rfd::FileDialog::new()
            .add_filter("Images", FILE_DIALOG_IMAGE_EXTS)
            .pick_files();
        let _ = tx.send(picked);
    });
}

pub(super) fn open_empty_slot_fill_dialog(window: &Window, ctx: &mut RenderContext, slot_id: u64) {
    if native_file_dialog_inflight(ctx) {
        return;
    }

    set_native_cursor_custom(window);
    let tab_id = ctx.active_tab_id();
    let (tx, rx) = std::sync::mpsc::channel();
    ctx.app_background.empty_slot_fill_dialog = Some(PendingEmptySlotFillDialog {
        tab_id,
        slot_id,
        rx,
    });
    std::thread::spawn(move || {
        let picked = rfd::FileDialog::new()
            .add_filter("Images", FILE_DIALOG_IMAGE_EXTS)
            .pick_file();
        let _ = tx.send(picked);
    });
}

pub(super) fn open_wallpaper_dialog(window: &Window, ctx: &mut RenderContext) {
    // Keep UI thread responsive while the native dialog is open.
    if native_file_dialog_inflight(ctx) {
        return;
    }

    set_native_cursor_custom(window);
    let (tx, rx) = std::sync::mpsc::channel();
    ctx.app_background.wallpaper_dialog_rx = Some(rx);
    std::thread::spawn(move || {
        let picked = rfd::FileDialog::new()
            .add_filter("Images", WALLPAPER_DIALOG_IMAGE_EXTS)
            .pick_file();
        let _ = tx.send(picked);
    });
}

fn native_file_dialog_inflight(ctx: &RenderContext) -> bool {
    ctx.app_background.canvas_import_dialog_rx.is_some()
        || ctx.app_background.empty_slot_fill_dialog.is_some()
        || ctx.app_background.wallpaper_dialog_rx.is_some()
}
