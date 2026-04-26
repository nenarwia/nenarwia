use std::sync::OnceLock;

use ::windows::Win32::UI::WindowsAndMessaging::HCURSOR;

static SYSTEM_CURSOR_HANDLES: OnceLock<SystemCursorHandles> = OnceLock::new();

#[derive(Clone, Copy)]
pub(super) struct SystemCursorHandles {
    pub(super) arrow: HCURSOR,
    pub(super) ew: HCURSOR,
    pub(super) ns: HCURSOR,
    pub(super) nesw: HCURSOR,
    pub(super) nwse: HCURSOR,
}

pub(super) fn system_cursor_handles() -> Option<SystemCursorHandles> {
    if let Some(handles) = SYSTEM_CURSOR_HANDLES.get().copied() {
        return Some(handles);
    }
    let loaded = load_system_cursor_handles()?;
    let _ = SYSTEM_CURSOR_HANDLES.set(loaded);
    SYSTEM_CURSOR_HANDLES.get().copied()
}

fn load_system_cursor_handles() -> Option<SystemCursorHandles> {
    use ::windows::Win32::UI::WindowsAndMessaging::{
        LoadCursorW, IDC_ARROW, IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE,
    };

    let load = |id, label: &str| match unsafe { LoadCursorW(None, id) } {
        Ok(cursor) => Some(cursor),
        Err(err) => {
            log::warn!("Failed to load system cursor '{label}': {err:?}");
            None
        }
    };

    Some(SystemCursorHandles {
        arrow: load(IDC_ARROW, "arrow")?,
        ew: load(IDC_SIZEWE, "size_we")?,
        ns: load(IDC_SIZENS, "size_ns")?,
        nesw: load(IDC_SIZENESW, "size_nesw")?,
        nwse: load(IDC_SIZENWSE, "size_nwse")?,
    })
}
