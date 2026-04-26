use winit::event_loop::EventLoopWindowTarget;
use winit::window::Window;

use crate::core::profiler::Profiler;
use crate::render::context::RenderContext;

use super::telemetry;
pub(super) fn handle_redraw_requested(
    window: &Window,
    ctx: &mut RenderContext,
    profiler: &mut Profiler,
    elwt: &EventLoopWindowTarget<()>,
) -> bool {
    let frame_now = std::time::Instant::now();
    ctx.viewport.set_content_bounds(ctx.scene.content_bounds());
    ctx.viewport.tick(frame_now);
    ctx.update_at(frame_now);
    ctx.sidebar_ui.is_animating();

    let mut rendered = false;
    match ctx.render() {
        Ok(_) => rendered = true,
        Err(wgpu::SurfaceError::Lost) => ctx.resize(ctx.gpu.size),
        Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
        Err(e) => log::error!("Render error: {:?}", e),
    }

    telemetry::update_frame_telemetry(window, ctx, profiler);

    rendered
}
