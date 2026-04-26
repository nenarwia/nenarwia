use crate::render::context::state::RenderContext;

pub fn update_camera_uniforms(ctx: &mut RenderContext) {
    let mut view = ctx.view();
    view.center.x = 0.0;
    view.center.y = 0.0;
    ctx.camera_uniform.update_view_proj(&view);
    ctx.gpu.queue.write_buffer(
        &ctx.camera_buffer,
        0,
        bytemuck::cast_slice(&[ctx.camera_uniform]),
    );
}
