use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::EventLoopWindowTarget;
use winit::window::Window;
use winit::window::WindowId;

use crate::core::engine::frame_pacing;
use crate::core::engine::window::refresh_platform_window_chrome;
use crate::core::profiler::Profiler;
use crate::render::context::RenderContext;
use crate::render::ui::UiAction;
use crate::spatial::navigation;

use super::client_resize;
use super::cursor_resize::{
    active_resize_direction_with_thickness, apply_cursor_for_current_position,
    apply_default_or_native_custom_cursor,
};
use super::redraw;
use super::resize;
use super::ui_actions;

pub struct HandleWindowEventInput<'a> {
    pub window: &'a Window,
    pub ctx: &'a mut RenderContext,
    pub profiler: &'a mut Profiler,
    pub window_shown: &'a mut bool,
    pub elwt: &'a EventLoopWindowTarget<()>,
}

pub fn handle_window_event(
    event: &WindowEvent,
    window_id: WindowId,
    input: HandleWindowEventInput<'_>,
) {
    let HandleWindowEventInput {
        window,
        ctx,
        profiler,
        window_shown,
        elwt,
    } = input;

    if window_id != window.id() {
        return;
    }
    if let WindowEvent::CursorMoved { position, .. } = event {
        ctx.cursor_pos = Some(*position);
        if client_resize::is_active(ctx) {
            client_resize::update(window, ctx);
            frame_pacing::request_redraw(ctx);
            return;
        }
        if ctx.wallpaper_preview_ui.is_visible() {
            apply_default_or_native_custom_cursor(window);
        } else {
            apply_cursor_for_current_position(window, ctx);
        }
        ui_actions::maybe_start_pending_titlebar_drag(window, ctx);
        frame_pacing::request_redraw(ctx);
    }
    if let WindowEvent::CursorEntered { .. } = event {
        if client_resize::is_active(ctx) {
            frame_pacing::request_redraw(ctx);
            return;
        }
        apply_default_or_native_custom_cursor(window);
        frame_pacing::request_redraw(ctx);
    }
    if let WindowEvent::CursorLeft { .. } = event {
        if client_resize::is_active(ctx) {
            frame_pacing::request_redraw(ctx);
            return;
        }
        ctx.cursor_pos = None;
        ctx.pending_titlebar_drag_origin = None;
        ctx.pending_canvas_click = None;
        ctx.last_media_click = None;
        ctx.window_chrome.clear_interaction_state();
        apply_default_or_native_custom_cursor(window);
        refresh_platform_window_chrome(window, ctx.window_fake_maximized);
        frame_pacing::request_redraw(ctx);
    }
    if let WindowEvent::Focused(focused) = event {
        if !*focused {
            client_resize::end(window, ctx);
        }
        ctx.mouse_left_down = false;
        ctx.pending_titlebar_drag_origin = None;
        ctx.pending_canvas_click = None;
        if !*focused {
            ctx.cursor_pos = None;
            ctx.last_media_click = None;
            ctx.window_chrome.clear_interaction_state();
        }
        apply_default_or_native_custom_cursor(window);
        refresh_platform_window_chrome(window, ctx.window_fake_maximized);
        frame_pacing::request_redraw(ctx);
    }

    if let Some(action) = ctx.handle_ui_hotkey(event) {
        if action != UiAction::Consume {
            ui_actions::apply_ui_action(action, window, ctx, elwt);
        }
        frame_pacing::request_redraw(ctx);
        return;
    }

    let mut left_press_pending_resize = false;
    if let WindowEvent::MouseInput {
        state,
        button: MouseButton::Left,
        ..
    } = event
    {
        ctx.mouse_left_down = *state == ElementState::Pressed;
        if *state == ElementState::Released {
            ctx.pending_titlebar_drag_origin = None;
            if client_resize::end(window, ctx) {
                apply_default_or_native_custom_cursor(window);
                refresh_platform_window_chrome(window, ctx.window_fake_maximized);
                frame_pacing::request_redraw(ctx);
                return;
            }
        }
        left_press_pending_resize = *state == ElementState::Pressed;
    }

    if left_press_pending_resize && !ctx.wallpaper_preview_ui.is_visible() {
        if let Some(direction) = active_resize_direction_with_thickness(
            window,
            ctx,
            resize::BORDER_RESIZE_RELEASE_THICKNESS_PX,
        ) {
            if client_resize::begin(window, ctx, direction) {
                apply_cursor_for_current_position(window, ctx);
            } else if let Err(err) = window.drag_resize_window(direction) {
                log::warn!("Window resize drag failed: {err:?}");
            }
            frame_pacing::request_redraw(ctx);
            return;
        }
    }

    if let Some(action) = ctx.handle_ui_event(event) {
        if action != UiAction::Consume {
            ui_actions::apply_ui_action(action, window, ctx, elwt);
        }
        if ctx.wallpaper_preview_ui.is_visible() {
            apply_default_or_native_custom_cursor(window);
        } else {
            apply_cursor_for_current_position(window, ctx);
        }
        frame_pacing::request_redraw(ctx);
        return;
    }

    let camera_changed = navigation::apply_window_event_with_cursor(
        &mut ctx.viewport,
        event,
        ctx.cursor_pos,
        std::time::Instant::now(),
    );

    if camera_changed {
        frame_pacing::request_redraw(ctx);
    }
    if !camera_changed {
        match event {
            WindowEvent::CloseRequested => {
                ctx.persist_tab_session();
                elwt.exit();
            }

            WindowEvent::Resized(size) => {
                if window_is_effectively_minimized(window, Some(*size)) {
                    return;
                }
                ctx.resize(*size);
                refresh_platform_window_chrome(window, ctx.window_fake_maximized);
                frame_pacing::request_redraw(ctx);
            }

            WindowEvent::RedrawRequested if !window_is_effectively_minimized(window, None) => {
                let rendered = redraw::handle_redraw_requested(window, ctx, profiler, elwt);
                if rendered && !*window_shown {
                    window.set_visible(true);
                    *window_shown = true;
                    frame_pacing::request_redraw(ctx);
                }
            }
            _ => {}
        }
    }
}

pub fn render_startup_frame(
    window: &Window,
    ctx: &mut RenderContext,
    profiler: &mut Profiler,
    elwt: &EventLoopWindowTarget<()>,
) -> bool {
    if window_is_effectively_minimized(window, None) {
        return false;
    }
    redraw::handle_redraw_requested(window, ctx, profiler, elwt)
}

pub(super) fn window_is_effectively_minimized(
    window: &Window,
    resized_size: Option<PhysicalSize<u32>>,
) -> bool {
    if let Some(minimized) = window.is_minimized() {
        return minimized;
    }
    let size = resized_size.unwrap_or_else(|| window.inner_size());
    size.width == 0 || size.height == 0
}
