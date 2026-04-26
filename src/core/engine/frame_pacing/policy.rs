use crate::render::context::state::FramePacingMode;
use crate::render::context::RenderContext;

pub fn initialize_context(ctx: &mut RenderContext) {
    sync_present_mode(ctx);
}

pub fn set_frame_pacing_mode(ctx: &mut RenderContext, mode: FramePacingMode) -> bool {
    if ctx.frame_pacing_mode == mode {
        return false;
    }
    ctx.frame_pacing_mode = mode;
    sync_present_mode(ctx);
    true
}

fn sync_present_mode(ctx: &mut RenderContext) {
    ctx.gpu
        .set_present_mode(present_mode_for(ctx.frame_pacing_mode));
}

pub fn present_mode_for(mode: FramePacingMode) -> wgpu::PresentMode {
    match mode {
        FramePacingMode::VSync => wgpu::PresentMode::Fifo,
        FramePacingMode::Unlimited => wgpu::PresentMode::AutoNoVsync,
    }
}

#[cfg(test)]
mod tests {
    use super::present_mode_for;
    use crate::render::context::state::FramePacingMode;

    #[test]
    fn vsync_maps_to_fifo_present_mode() {
        assert_eq!(
            present_mode_for(FramePacingMode::VSync),
            wgpu::PresentMode::Fifo
        );
    }

    #[test]
    fn unlimited_maps_to_auto_no_vsync_present_mode() {
        assert_eq!(
            present_mode_for(FramePacingMode::Unlimited),
            wgpu::PresentMode::AutoNoVsync
        );
    }
}
