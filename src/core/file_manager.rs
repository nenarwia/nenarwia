use std::path::{Path, PathBuf};

pub fn reveal_path(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("Path '{}' does not exist.", path.display()));
    }

    let resolved = resolve_reveal_path(path);
    reveal_path_impl(&resolved)
}

#[cfg(target_os = "windows")]
fn resolve_reveal_path(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };

    normalize_windows_shell_path(&absolute)
}

#[cfg(not(target_os = "windows"))]
fn resolve_reveal_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(target_os = "windows")]
fn reveal_path_impl(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        return std::process::Command::new("explorer.exe")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|err| format!("Failed to open '{}' in Explorer: {}", path.display(), err));
    }

    use std::ptr::null_mut;

    use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
    use windows::Win32::System::Com::{
        CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::Common::ITEMIDLIST;
    use windows::Win32::UI::Shell::{ILClone, ILFindLastID, ILFree, SHOpenFolderAndSelectItems};

    let folder_path = path.parent().ok_or_else(|| {
        format!(
            "Path '{}' does not have a parent folder for Explorer reveal.",
            path.display()
        )
    })?;

    let com_initialized = unsafe {
        match CoInitializeEx(None, COINIT_APARTMENTTHREADED) {
            Ok(_) => true,
            Err(err) if err.code() == RPC_E_CHANGED_MODE => false,
            Err(err) => {
                return Err(format!(
                    "Failed to initialize COM for Explorer reveal '{}': {}",
                    path.display(),
                    err
                ));
            }
        }
    };

    let result = unsafe {
        let mut folder_pidl: *mut ITEMIDLIST = null_mut();
        let mut item_pidl: *mut ITEMIDLIST = null_mut();
        let mut child_pidl: *mut ITEMIDLIST = null_mut();

        let reveal = (|| {
            folder_pidl = parse_shell_path(folder_path)?;
            item_pidl = parse_shell_path(path)?;

            let last_id = ILFindLastID(item_pidl.cast_const());
            if last_id.is_null() {
                return Err(format!(
                    "Explorer did not return a child item ID for '{}'.",
                    path.display()
                ));
            }

            child_pidl = ILClone(last_id.cast_const());
            if child_pidl.is_null() {
                return Err(format!(
                    "Explorer failed to clone the selected child item for '{}'.",
                    path.display()
                ));
            }

            let selection = [child_pidl.cast_const()];
            SHOpenFolderAndSelectItems(folder_pidl.cast_const(), Some(&selection), 0).map_err(
                |err| format!("Failed to reveal '{}' in Explorer: {}", path.display(), err),
            )
        })();

        if !child_pidl.is_null() {
            ILFree(Some(child_pidl.cast_const()));
        }
        if !item_pidl.is_null() {
            CoTaskMemFree(Some(item_pidl.cast()));
        }
        if !folder_pidl.is_null() {
            CoTaskMemFree(Some(folder_pidl.cast()));
        }
        if com_initialized {
            CoUninitialize();
        }

        reveal
    };

    result
}

#[cfg(target_os = "windows")]
fn normalize_windows_shell_path(path: &Path) -> PathBuf {
    use std::path::{Component, Prefix};

    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut components = canonical.components();
    let Some(first) = components.next() else {
        return canonical;
    };

    match first {
        Component::Prefix(prefix_component) => match prefix_component.kind() {
            Prefix::VerbatimDisk(letter) => {
                let mut normalized = PathBuf::from(format!("{}:\\", letter as char));
                for component in components {
                    normalized.push(component.as_os_str());
                }
                normalized
            }
            Prefix::VerbatimUNC(server, share) => {
                let mut normalized = PathBuf::from(format!(
                    "\\\\{}\\{}",
                    server.to_string_lossy(),
                    share.to_string_lossy()
                ));
                for component in components {
                    normalized.push(component.as_os_str());
                }
                normalized
            }
            _ => canonical,
        },
        _ => canonical,
    }
}

#[cfg(target_os = "windows")]
fn parse_shell_path(
    path: &Path,
) -> Result<*mut windows::Win32::UI::Shell::Common::ITEMIDLIST, String> {
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;

    use windows::core::PCWSTR;
    use windows::Win32::System::Com::IBindCtx;
    use windows::Win32::UI::Shell::Common::ITEMIDLIST;
    use windows::Win32::UI::Shell::SHParseDisplayName;

    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);

    let mut pidl: *mut ITEMIDLIST = null_mut();
    unsafe {
        SHParseDisplayName(
            PCWSTR(wide.as_ptr()),
            Option::<&IBindCtx>::None,
            &mut pidl,
            0,
            None,
        )
    }
    .map_err(|err| {
        format!(
            "Failed to parse Explorer path '{}': {}",
            path.display(),
            err
        )
    })?;

    if pidl.is_null() {
        return Err(format!(
            "Explorer returned an empty PIDL for '{}'.",
            path.display()
        ));
    }

    Ok(pidl)
}

#[cfg(target_os = "macos")]
fn reveal_path_impl(path: &Path) -> Result<(), String> {
    std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("Failed to reveal '{}' in Finder: {}", path.display(), err))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn reveal_path_impl(path: &Path) -> Result<(), String> {
    let target = path.parent().unwrap_or(path);
    std::process::Command::new("xdg-open")
        .arg(target)
        .spawn()
        .map(|_| ())
        .map_err(|err| {
            format!(
                "Failed to open '{}' in file manager: {}",
                target.display(),
                err
            )
        })
}

#[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
fn reveal_path_impl(path: &Path) -> Result<(), String> {
    Err(format!(
        "Opening '{}' in a file manager is not supported on this platform.",
        path.display()
    ))
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "windows")]
    #[test]
    fn windows_reveal_path_stays_shell_friendly() {
        let root = unique_test_dir("reveal_path");
        std::fs::create_dir_all(&root).expect("create temp dir");
        let path = root.join("space test.png");
        std::fs::write(&path, b"test").expect("write temp file");
        let resolved = super::resolve_reveal_path(&path);

        assert!(resolved.is_absolute());
        assert!(!resolved.to_string_lossy().starts_with(r"\\?\"));

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_reveal_path_accepts_directory_targets() {
        let root = unique_test_dir("reveal_dir");
        std::fs::create_dir_all(&root).expect("create temp dir");

        let resolved = super::resolve_reveal_path(&root);

        assert!(resolved.is_absolute());
        assert!(!resolved.to_string_lossy().starts_with(r"\\?\"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(target_os = "windows")]
    fn unique_test_dir(label: &str) -> std::path::PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};

        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("nenarwia_{label}_{suffix}"))
    }
}
