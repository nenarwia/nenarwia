use std::sync::Arc;
use winit::window::Window;

use super::{assemble, bootstrap, config, resources};
use crate::core::app_settings;
use crate::render::context::state::RenderContext;
use crate::render::gpu::GpuState;

impl RenderContext {
    pub async fn new(window: Arc<Window>) -> Self {
        // Initialize GPU state.
        let gpu = GpuState::new(window.clone()).await;
        let max_dim = gpu.device.limits().max_texture_dimension_2d;

        config::log_runtime_environment();
        crate::core::color::prewarm_gpu_resize_backend();
        let tuning = config::load_tuning();
        let graphics_backend_preference =
            app_settings::windows_graphics_backend_preference_for_ui();
        config::log_tuning_summary(tuning);

        let cache_resources = resources::create_cache_init_resources(&gpu, max_dim);
        let bootstrap = bootstrap::build_bootstrap_state();
        let render_resources = resources::create_render_init_resources(
            &gpu,
            &cache_resources.bindings.camera_layout,
            &cache_resources.bindings.texture_layout,
            cache_resources.gpu_feedback.as_ref(),
        );
        let mut ctx = assemble::build_context(assemble::BuildContextInput {
            window,
            gpu,
            graphics_backend_preference,
            tuning,
            cache_resources,
            bootstrap,
            render_resources,
        });

        ctx.initialize_saved_wallpapers();
        ctx.restore_tab_session_or_initialize();
        ctx.auto_frame_scene_if_needed();
        ctx
    }
}
