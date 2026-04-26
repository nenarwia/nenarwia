use std::path::Path;

pub fn move_path_to_trash(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        move_path_to_trash_windows(path)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        Err("Move to Trash is only implemented on Windows.".to_string())
    }
}

#[cfg(target_os = "windows")]
fn move_path_to_trash_windows(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::{
        SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FO_DELETE,
        SHFILEOPSTRUCTW,
    };

    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    wide.push(0);

    let mut op = SHFILEOPSTRUCTW {
        hwnd: HWND(0),
        wFunc: FO_DELETE,
        pFrom: PCWSTR(wide.as_ptr()),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT).0 as u16,
        ..Default::default()
    };

    let code = unsafe { SHFileOperationW(&mut op) };
    if code != 0 {
        if !path.exists() {
            return Ok(());
        }
        return Err(format!(
            "Windows recycle-bin delete failed for '{}': code {}",
            path.display(),
            code
        ));
    }
    if op.fAnyOperationsAborted.as_bool() {
        return Err(format!(
            "Windows recycle-bin delete was aborted for '{}'.",
            path.display()
        ));
    }

    Ok(())
}
