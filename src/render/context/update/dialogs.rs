use std::sync::mpsc::TryRecvError;

use crate::render::context::state::RenderContext;

impl RenderContext {
    pub(super) fn poll_canvas_import_dialog_result(&mut self) {
        let Some(rx) = self.app_background.canvas_import_dialog_rx.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(picked) => {
                self.app_background.canvas_import_dialog_rx = None;
                if let Some(paths) = picked {
                    if let Err(err) = self.validate_manual_canvas_import_paths(paths.as_slice()) {
                        log::warn!("Rejected picked media import onto canvas: {err}");
                        return;
                    }
                    if let Err(err) = self.add_media_to_canvas_from_paths(paths.as_slice()) {
                        log::warn!("Failed to import picked media onto canvas: {err}");
                    }
                }
            }
            Err(TryRecvError::Disconnected) => {
                self.app_background.canvas_import_dialog_rx = None;
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    pub(super) fn poll_empty_slot_fill_dialog_result(&mut self) {
        let Some(dialog) = self.app_background.empty_slot_fill_dialog.as_ref() else {
            return;
        };
        match dialog.rx.try_recv() {
            Ok(picked) => {
                let tab_id = dialog.tab_id;
                let slot_id = dialog.slot_id;
                self.app_background.empty_slot_fill_dialog = None;
                if self.active_tab_id() != tab_id {
                    return;
                }
                if let Some(path) = picked {
                    if let Err(err) = self.validate_manual_canvas_fill_path(&path) {
                        log::warn!(
                            "Rejected empty slot fill from '{}': {}",
                            path.display(),
                            err
                        );
                        return;
                    }
                    if let Err(err) = self.fill_empty_canvas_slot_from_path(slot_id, &path) {
                        log::warn!(
                            "Failed to fill empty slot {} from '{}': {}",
                            slot_id,
                            path.display(),
                            err
                        );
                    }
                }
            }
            Err(TryRecvError::Disconnected) => {
                self.app_background.empty_slot_fill_dialog = None;
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    pub(super) fn poll_wallpaper_dialog_result(&mut self) {
        let Some(rx) = self.app_background.wallpaper_dialog_rx.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(picked) => {
                self.app_background.wallpaper_dialog_rx = None;
                if let Some(path) = picked {
                    if let Err(err) = self.open_wallpaper_preview_from_path(&path) {
                        log::warn!(
                            "Failed to open wallpaper preview for '{}': {}",
                            path.display(),
                            err
                        );
                    }
                }
            }
            Err(TryRecvError::Disconnected) => {
                self.app_background.wallpaper_dialog_rx = None;
            }
            Err(TryRecvError::Empty) => {}
        }
    }
}
