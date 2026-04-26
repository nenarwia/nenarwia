use winit::window::Window;

#[derive(Clone)]
pub(super) struct AppIcons {
    pub(super) window: winit::window::Icon,
    pub(super) taskbar: winit::window::Icon,
}

pub(super) fn load_app_icons() -> Option<AppIcons> {
    use std::path::PathBuf;
    use winit::dpi::PhysicalSize;
    use winit::platform::windows::IconExtWindows;

    let window_size = PhysicalSize::new(32, 32);
    let taskbar_size = PhysicalSize::new(256, 256);

    // Prefer embedded exe resource to avoid runtime filesystem dependency and shell delays.
    let window_from_res = winit::window::Icon::from_resource(1, Some(window_size));
    let taskbar_from_res = winit::window::Icon::from_resource(1, Some(taskbar_size));
    if let (Ok(window), Ok(taskbar)) = (window_from_res, taskbar_from_res) {
        return Some(AppIcons { window, taskbar });
    }

    // Fallback for environments where resource embedding is unavailable.
    let icon_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("app.ico");
    let window_from_path = winit::window::Icon::from_path(&icon_path, Some(window_size));
    let taskbar_from_path = winit::window::Icon::from_path(&icon_path, Some(taskbar_size));
    if let (Ok(window), Ok(taskbar)) = (window_from_path, taskbar_from_path) {
        return Some(AppIcons { window, taskbar });
    }

    let image = match image::load_from_memory(include_bytes!("../../../../assets/app.ico")) {
        Ok(img) => img.to_rgba8(),
        Err(err) => {
            log::warn!("Failed to decode app icon: {err:?}");
            return None;
        }
    };
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        log::warn!("App icon has invalid dimensions: {width}x{height}");
        return None;
    }
    match winit::window::Icon::from_rgba(image.into_raw(), width, height) {
        Ok(icon) => Some(AppIcons {
            window: icon.clone(),
            taskbar: icon,
        }),
        Err(err) => {
            log::warn!("Failed to create winit icon from app icon image: {err:?}");
            None
        }
    }
}

pub(super) fn apply_native_app_icon_handles(window: &Window) {
    use std::os::windows::ffi::OsStrExt;
    use std::path::PathBuf;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        LoadImageW, SendMessageW, SetClassLongPtrW, GCLP_HICON, GCLP_HICONSM, HICON, ICON_BIG,
        ICON_SMALL, IMAGE_ICON, LR_LOADFROMFILE, LR_SHARED, WM_SETICON,
    };
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let hwnd = match window.window_handle() {
        Ok(handle) => match handle.as_raw() {
            RawWindowHandle::Win32(win32) => HWND(win32.hwnd.get()),
            _ => return,
        },
        Err(_) => return,
    };

    let icon_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("app.ico");
    let mut icon_path_utf16: Vec<u16> = icon_path.as_os_str().encode_wide().collect();
    icon_path_utf16.push(0);
    let icon_pcwstr = PCWSTR(icon_path_utf16.as_ptr());

    let load_icon = |size: i32| unsafe {
        LoadImageW(
            None,
            icon_pcwstr,
            IMAGE_ICON,
            size,
            size,
            LR_LOADFROMFILE | LR_SHARED,
        )
        .map(|handle| HICON(handle.0))
    };

    let small_icon = load_icon(32);
    let big_icon = load_icon(256);

    if let Ok(icon) = small_icon {
        unsafe {
            let _ = SetClassLongPtrW(hwnd, GCLP_HICONSM, icon.0);
            let _ = SendMessageW(
                hwnd,
                WM_SETICON,
                WPARAM(ICON_SMALL as usize),
                LPARAM(icon.0),
            );
        }
    } else if let Err(err) = small_icon {
        log::warn!(
            "Failed to load small native app icon from {}: {err:?}",
            icon_path.display()
        );
    }

    if let Ok(icon) = big_icon {
        unsafe {
            let _ = SetClassLongPtrW(hwnd, GCLP_HICON, icon.0);
            let _ = SendMessageW(hwnd, WM_SETICON, WPARAM(ICON_BIG as usize), LPARAM(icon.0));
        }
    } else if let Err(err) = big_icon {
        log::warn!(
            "Failed to load big native app icon from {}: {err:?}",
            icon_path.display()
        );
    }
}
