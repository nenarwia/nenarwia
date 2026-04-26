use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event_loop::EventLoopWindowTarget;
use winit::window::{Fullscreen, Window};

use crate::core::engine::window::refresh_platform_window_chrome;
use crate::render::context::state::WindowedPlacement;
use crate::render::context::RenderContext;

#[derive(Clone, Copy, Debug)]
struct WorkArea {
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
}

pub(super) fn close_window_and_exit(ctx: &mut RenderContext, elwt: &EventLoopWindowTarget<()>) {
    ctx.persist_tab_session();
    elwt.exit();
}

pub(super) fn toggle_window_maximize(window: &Window, ctx: &mut RenderContext) {
    if window.fullscreen().is_some() {
        window.set_fullscreen(None);
        ctx.window_was_maximized_before_fullscreen = false;
        ctx.window_was_fake_maximized_before_fullscreen = false;
    }

    if ctx.window_fake_maximized || window.is_maximized() {
        ctx.window_fake_maximized = false;
        restore_windowed_placement(window, ctx);
        refresh_platform_window_chrome(window, ctx.window_fake_maximized);
        return;
    }

    remember_windowed_placement(window, ctx);
    ctx.window_fake_maximized = apply_fake_maximize(window);
    if !ctx.window_fake_maximized {
        log::warn!("Failed to resolve monitor work area for fake maximize.");
    }
    refresh_platform_window_chrome(window, ctx.window_fake_maximized);
}

pub(super) fn toggle_window_fullscreen(window: &Window, ctx: &mut RenderContext) {
    if window.fullscreen().is_some() {
        window.set_fullscreen(None);
        if ctx.window_was_fake_maximized_before_fullscreen
            || ctx.window_was_maximized_before_fullscreen
        {
            ctx.window_fake_maximized = apply_fake_maximize(window);
            if !ctx.window_fake_maximized {
                log::warn!("Failed to restore fake maximize after fullscreen.");
            }
        } else {
            ctx.window_fake_maximized = false;
            restore_windowed_placement(window, ctx);
        }
        ctx.window_was_maximized_before_fullscreen = false;
        ctx.window_was_fake_maximized_before_fullscreen = false;
    } else {
        ctx.window_was_fake_maximized_before_fullscreen = ctx.window_fake_maximized;
        ctx.window_was_maximized_before_fullscreen = window.is_maximized();
        remember_windowed_placement(window, ctx);
        ctx.window_fake_maximized = false;
        if ctx.window_was_maximized_before_fullscreen {
            // Drop out of the maximized work-area path before switching to borderless fullscreen,
            // otherwise Windows can preserve a taskbar-sized strip on the first fullscreen frame.
            window.set_maximized(false);
        }
        window.set_fullscreen(Some(Fullscreen::Borderless(None)));
    }
    refresh_platform_window_chrome(window, ctx.window_fake_maximized);
}

fn remember_windowed_placement(window: &Window, ctx: &mut RenderContext) {
    if window.fullscreen().is_some() || window.is_maximized() || ctx.window_fake_maximized {
        return;
    }

    ctx.windowed_placement = Some(WindowedPlacement {
        position: window.outer_position().ok(),
        size: window.inner_size(),
    });
}

fn restore_windowed_placement(window: &Window, ctx: &RenderContext) {
    window.set_maximized(false);
    let Some(placement) = ctx.windowed_placement else {
        return;
    };

    let _ = window.request_inner_size(placement.size);
    if let Some(position) = placement.position {
        window.set_outer_position(position);
    }
}

fn apply_fake_maximize(window: &Window) -> bool {
    let Some(work_area) = current_monitor_work_area(window) else {
        return false;
    };

    window.set_maximized(false);
    window.set_outer_position(work_area.position);
    let _ = window.request_inner_size(work_area.size);
    true
}

#[cfg(target_os = "windows")]
fn current_monitor_work_area(window: &Window) -> Option<WorkArea> {
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let hwnd = match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(win32) => HWND(win32.hwnd.get()),
        _ => return None,
    };
    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    if monitor.0 == 0 {
        return None;
    }

    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        rcMonitor: RECT::default(),
        rcWork: RECT::default(),
        dwFlags: 0,
    };
    if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        return None;
    }

    rect_to_work_area(info.rcWork)
}

#[cfg(not(target_os = "windows"))]
fn current_monitor_work_area(window: &Window) -> Option<WorkArea> {
    let monitor = window.current_monitor()?;
    Some(WorkArea {
        position: monitor.position(),
        size: monitor.size(),
    })
}

fn rect_to_work_area(rect: windows::Win32::Foundation::RECT) -> Option<WorkArea> {
    let width = rect.right.checked_sub(rect.left)?;
    let height = rect.bottom.checked_sub(rect.top)?;
    if width <= 0 || height <= 0 {
        return None;
    }

    Some(WorkArea {
        position: PhysicalPosition::new(rect.left, rect.top),
        size: PhysicalSize::new(width as u32, height as u32),
    })
}
