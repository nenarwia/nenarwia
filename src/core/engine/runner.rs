use winit::event::{Event, WindowEvent};
use winit::event_loop::{EventLoop, EventLoopBuilder};

use super::frame_pacing;
use super::runloop;
use super::window::{create_window, refresh_platform_window_chrome};
use crate::core::profiler::Profiler;
use crate::render::context::RenderContext;

pub struct CanvasEngine;

impl CanvasEngine {
    pub fn run() {
        #[cfg(target_os = "windows")]
        maybe_attach_parent_console_for_logs();
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
        log::info!("Starting Canvas Engine...");
        #[cfg(target_os = "windows")]
        set_windows_app_user_model_id();

        let mut event_loop_builder = EventLoopBuilder::new();
        let event_loop: EventLoop<()> = event_loop_builder.build().unwrap();
        let window = create_window(&event_loop);

        // Defer heavy GPU/context initialization until the event loop is pumping
        // so the OS can present the window/icon immediately.
        let mut ctx: Option<RenderContext> = None;
        let mut profiler = Profiler::new();
        #[cfg(target_os = "windows")]
        let mut window_shown = false;
        #[cfg(not(target_os = "windows"))]
        let mut window_shown = true;

        log::info!("Window created. Starting event loop...");
        window.request_redraw();

        event_loop
            .run(move |event, elwt| {
                // Sleep by default until next event to avoid busy looping.
                frame_pacing::set_idle_control_flow(elwt);

                match event {
                    Event::WindowEvent {
                        ref event,
                        window_id,
                    } => {
                        if let Some(ctx) = ctx.as_mut() {
                            runloop::handle_window_event(
                                event,
                                window_id,
                                runloop::HandleWindowEventInput {
                                    window: &window,
                                    ctx,
                                    profiler: &mut profiler,
                                    window_shown: &mut window_shown,
                                    elwt,
                                },
                            );
                        } else if matches!(event, WindowEvent::CloseRequested) {
                            elwt.exit();
                        }
                    }

                    Event::AboutToWait => {
                        if ctx.is_none() {
                            ctx = Some(pollster::block_on(RenderContext::new(window.clone())));
                            if let Some(ctx) = ctx.as_mut() {
                                frame_pacing::initialize_context(ctx);
                            }
                            log::info!("Engine initialized. Running loop...");
                            if let Some(ctx) = ctx.as_mut() {
                                runloop::render_startup_frame(&window, ctx, &mut profiler, elwt);
                            }
                            #[cfg(target_os = "windows")]
                            if !window_shown {
                                window.set_decorations(false);
                                refresh_platform_window_chrome(&window, false);
                                window.set_visible(true);
                                window_shown = true;
                                if let Some(ctx) = ctx.as_mut() {
                                    runloop::render_startup_frame(
                                        &window,
                                        ctx,
                                        &mut profiler,
                                        elwt,
                                    );
                                }
                            }
                            window.request_redraw();
                        }

                        if let Some(ctx) = ctx.as_mut() {
                            // Keep rendering while background streaming work is pending.
                            let continuous_redraw = runloop::check_background_tasks(ctx, &window);
                            frame_pacing::service_redraws(&window, ctx, elwt, continuous_redraw);
                        }
                    }

                    _ => {}
                }
            })
            .expect("Loop error");
    }
}

#[cfg(target_os = "windows")]
fn maybe_attach_parent_console_for_logs() {
    use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};

    if !should_attach_parent_console_for_logs() {
        return;
    }

    // Ignore errors here:
    // - already attached to a console,
    // - no parent console (started from Explorer), etc.
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(target_os = "windows")]
fn should_attach_parent_console_for_logs() -> bool {
    env_flag_on("CANVAS_STAGE0_LOG")
        || env_flag_on("CANVAS_UI_METRICS")
        || std::env::var_os("RUST_LOG").is_some()
}

#[cfg(target_os = "windows")]
fn env_flag_on(name: &str) -> bool {
    let val = std::env::var(name).unwrap_or_default().to_lowercase();
    matches!(val.as_str(), "1" | "true" | "yes" | "on")
}

#[cfg(target_os = "windows")]
fn set_windows_app_user_model_id() {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;

    const APP_USER_MODEL_ID: &str = "CanvasEngine.App";
    let mut app_id_utf16: Vec<u16> = APP_USER_MODEL_ID.encode_utf16().collect();
    app_id_utf16.push(0);

    let result = unsafe { SetCurrentProcessExplicitAppUserModelID(PCWSTR(app_id_utf16.as_ptr())) };
    if let Err(err) = result {
        log::warn!("Failed to set explicit AppUserModelID ({APP_USER_MODEL_ID}): {err:?}");
    }
}
