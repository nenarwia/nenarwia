use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use winit::dpi::PhysicalPosition;
use winit::window::Window;

use crate::render::context::state::RenderContext;

const SYSTEM_FILE_DROP_DEBOUNCE: Duration = Duration::from_millis(50);

fn should_process_system_file_drop(paths: &[PathBuf], drop_on_canvas: bool) -> bool {
    drop_on_canvas && !paths.is_empty()
}

impl RenderContext {
    pub(super) fn flush_stale_system_file_drop_impl(&mut self) {
        if !self
            .background
            .system_file_drop
            .should_finalize(Instant::now(), SYSTEM_FILE_DROP_DEBOUNCE)
        {
            return;
        }
        self.finalize_system_file_drop_batch();
    }

    pub(super) fn queue_system_file_drop(&mut self, path: &Path) {
        let on_canvas_if_first = self.background.system_file_drop.paths.is_empty()
            && self.system_file_drop_targets_canvas();
        self.background
            .system_file_drop
            .queue(path, Instant::now(), on_canvas_if_first);
    }

    pub(super) fn finalize_system_file_drop_batch(&mut self) {
        let (paths, drop_on_canvas) = self.background.system_file_drop.take_batch();

        if !should_process_system_file_drop(paths.as_slice(), drop_on_canvas) {
            return;
        }

        if let Err(err) = self.validate_manual_canvas_import_paths(paths.as_slice()) {
            log::warn!("Rejected system file drop onto canvas: {err}");
            return;
        }

        if let Err(err) = self.add_media_to_canvas_from_paths(paths.as_slice()) {
            log::warn!("Failed to import system file drop onto canvas: {}", err);
        } else {
            log::info!("Queued system file drop for canvas import.");
        }
    }

    fn system_file_drop_targets_canvas(&self) -> bool {
        if self.wallpaper_preview_ui.is_visible() {
            return false;
        }

        self.resolved_system_file_drop_position()
            .map(|pos| !self.cursor_over_canvas_blocking_ui(pos))
            .unwrap_or(true)
    }

    fn resolved_system_file_drop_position(&self) -> Option<PhysicalPosition<f64>> {
        query_window_cursor_position(self.window.as_ref()).or(self.cursor_pos)
    }
}

#[cfg(target_os = "windows")]
fn query_window_cursor_position(window: &Window) -> Option<PhysicalPosition<f64>> {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    use windows::Win32::Foundation::{HWND, POINT};
    use windows::Win32::Graphics::Gdi::ScreenToClient;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let hwnd = match window.window_handle() {
        Ok(handle) => match handle.as_raw() {
            RawWindowHandle::Win32(win32) => HWND(win32.hwnd.get()),
            _ => return None,
        },
        Err(_) => return None,
    };

    let mut point = POINT::default();
    if unsafe { GetCursorPos(&mut point).is_err() } {
        return None;
    }
    if unsafe { !ScreenToClient(hwnd, &mut point).as_bool() } {
        return None;
    }

    Some(PhysicalPosition::new(point.x as f64, point.y as f64))
}

#[cfg(not(target_os = "windows"))]
fn query_window_cursor_position(_window: &Window) -> Option<PhysicalPosition<f64>> {
    None
}

#[cfg(test)]
mod tests {
    use super::{should_process_system_file_drop, SYSTEM_FILE_DROP_DEBOUNCE};
    use crate::render::context::document::SystemFileDropState;
    use std::path::Path;
    use std::time::{Duration, Instant};

    #[test]
    fn queue_deduplicates_paths_within_batch() {
        let mut state = SystemFileDropState::default();
        let now = Instant::now();

        assert!(state.queue(Path::new("a.png"), now, true));
        assert!(!state.queue(Path::new("a.png"), now + Duration::from_millis(5), false));
        assert_eq!(state.paths.len(), 1);
    }

    #[test]
    fn first_drop_snapshots_canvas_target_for_whole_batch() {
        let mut state = SystemFileDropState::default();
        let now = Instant::now();

        state.queue(Path::new("a.png"), now, false);
        state.queue(Path::new("b.png"), now + Duration::from_millis(5), true);

        assert!(!state.on_canvas);
    }

    #[test]
    fn debounce_waits_until_timeout_before_flushing() {
        let mut state = SystemFileDropState::default();
        let now = Instant::now();
        state.queue(Path::new("a.png"), now, true);

        assert!(!state.should_finalize(
            now + SYSTEM_FILE_DROP_DEBOUNCE - Duration::from_millis(1),
            SYSTEM_FILE_DROP_DEBOUNCE,
        ));
        assert!(state.should_finalize(now + SYSTEM_FILE_DROP_DEBOUNCE, SYSTEM_FILE_DROP_DEBOUNCE));
    }

    #[test]
    fn finalize_is_noop_for_empty_or_non_canvas_batches() {
        assert!(!should_process_system_file_drop(&[], true));
        assert!(!should_process_system_file_drop(
            &[Path::new("a.png").to_path_buf()],
            false
        ));
        assert!(should_process_system_file_drop(
            &[Path::new("a.png").to_path_buf()],
            true
        ));
    }
}
