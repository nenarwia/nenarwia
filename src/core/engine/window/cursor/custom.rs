use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use winit::window::Window;

use ::windows::Win32::UI::WindowsAndMessaging::HCURSOR;

use super::apply::apply_native_cursor_to_window;
use super::mode;
use super::system::system_cursor_handles;

static NATIVE_CURSOR_HANDLE: OnceLock<HCURSOR> = OnceLock::new();
static NATIVE_CURSOR_INSTALLED: AtomicBool = AtomicBool::new(false);

pub fn install_native_custom_cursor(window: &Window) -> bool {
    if let Some(cursor) = custom_cursor_handle() {
        NATIVE_CURSOR_INSTALLED.store(true, Ordering::Relaxed);
        mode::set_custom_mode();
        apply_native_cursor_to_window(window, cursor);
        return true;
    }

    let Some(hcursor) = create_native_custom_cursor() else {
        return false;
    };

    let _ = NATIVE_CURSOR_HANDLE.set(hcursor);
    NATIVE_CURSOR_INSTALLED.store(true, Ordering::Relaxed);
    mode::set_custom_mode();
    apply_native_cursor_to_window(window, hcursor);
    let _ = system_cursor_handles();
    true
}

pub(super) fn custom_cursor_handle() -> Option<HCURSOR> {
    NATIVE_CURSOR_HANDLE.get().copied()
}

fn create_native_custom_cursor() -> Option<HCURSOR> {
    use ::windows::Win32::Foundation::BOOL;
    use ::windows::Win32::Graphics::Gdi::{CreateBitmap, DeleteObject};
    use ::windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, ICONINFO};

    let image = match image::load_from_memory(include_bytes!(
        "../../../../render/ui/assets/cursor_default.png"
    )) {
        Ok(img) => img.to_rgba8(),
        Err(err) => {
            log::warn!("Failed to decode native cursor image: {err:?}");
            return None;
        }
    };
    let width = image.width();
    let height = image.height();
    if width == 0 || height == 0 {
        return None;
    }
    let rgba = image.into_raw();
    let bgra_pixels = rgba_to_bgra(&rgba, width, height);

    let mask_stride = ((width as usize + 31) / 32) * 4;
    let mask_bits = vec![0u8; mask_stride * height as usize];

    let hbm_color = unsafe {
        CreateBitmap(
            width as i32,
            height as i32,
            1,
            32,
            Some(bgra_pixels.as_ptr().cast()),
        )
    };
    if hbm_color.0 == 0 {
        log::warn!("Failed to create native cursor color bitmap.");
        return None;
    }

    let hbm_mask = unsafe {
        CreateBitmap(
            width as i32,
            height as i32,
            1,
            1,
            Some(mask_bits.as_ptr().cast()),
        )
    };
    if hbm_mask.0 == 0 {
        unsafe {
            let _ = DeleteObject(hbm_color);
        }
        log::warn!("Failed to create native cursor mask bitmap.");
        return None;
    }

    let icon_info = ICONINFO {
        fIcon: BOOL(0),
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: hbm_mask,
        hbmColor: hbm_color,
    };
    let hicon = unsafe { CreateIconIndirect(&icon_info) };

    unsafe {
        let _ = DeleteObject(hbm_mask);
        let _ = DeleteObject(hbm_color);
    }

    match hicon {
        Ok(icon) => Some(HCURSOR(icon.0)),
        Err(err) => {
            log::warn!("Failed to create native custom cursor: {err:?}");
            None
        }
    }
}

fn rgba_to_bgra(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut bgra_pixels = vec![0u8; (width as usize) * (height as usize) * 4];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let src_idx = (y * width as usize + x) * 4;
            let dst_idx = (y * width as usize + x) * 4;
            let r = rgba[src_idx];
            let g = rgba[src_idx + 1];
            let b = rgba[src_idx + 2];
            let a = rgba[src_idx + 3];
            bgra_pixels[dst_idx] = b;
            bgra_pixels[dst_idx + 1] = g;
            bgra_pixels[dst_idx + 2] = r;
            bgra_pixels[dst_idx + 3] = a;
        }
    }
    bgra_pixels
}
