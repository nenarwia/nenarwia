use std::sync::Arc;

use winit::{
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use super::chrome::refresh_platform_window_chrome;
#[cfg(target_os = "windows")]
use super::cursor::install_native_cursor_wndproc_subclass;
use super::cursor::install_native_custom_cursor;
#[cfg(target_os = "windows")]
use super::icons::{apply_native_app_icon_handles, load_app_icons};

pub fn create_window(event_loop: &EventLoop<()>) -> Arc<Window> {
    #[cfg(target_os = "windows")]
    let app_icons = load_app_icons();

    let mut builder = WindowBuilder::new()
        .with_title("nenarwia")
        .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0))
        .with_decorations(false)
        .with_resizable(true)
        .with_visible(false);
    #[cfg(target_os = "windows")]
    {
        use winit::platform::windows::WindowBuilderExtWindows;
        if let Some(icon) = app_icons.as_ref() {
            builder = builder
                .with_window_icon(Some(icon.window.clone()))
                .with_taskbar_icon(Some(icon.taskbar.clone()));
        }
    }

    let window = builder.build(event_loop).expect("Failed to create window");
    #[cfg(target_os = "windows")]
    if let Some(icon) = app_icons {
        use winit::platform::windows::WindowExtWindows;
        window.set_window_icon(Some(icon.window));
        window.set_taskbar_icon(Some(icon.taskbar));
    }
    #[cfg(target_os = "windows")]
    apply_native_app_icon_handles(&window);
    refresh_platform_window_chrome(&window, false);
    install_native_custom_cursor(&window);
    #[cfg(target_os = "windows")]
    install_native_cursor_wndproc_subclass(&window);
    #[cfg(not(target_os = "windows"))]
    window.set_visible(true);
    Arc::new(window)
}
