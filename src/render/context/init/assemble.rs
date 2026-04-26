use std::collections::HashMap;
use std::sync::Arc;
use winit::window::Window;

use super::bootstrap::BootstrapInitState;
use super::config::{self, InitTuning};
use super::resources::{CacheInitResources, RenderInitResources};
use crate::core::app_settings::GraphicsBackendPreference;
use crate::render::context::document::{
    ActiveDocumentState, CanvasDocumentMode, DocumentBackgroundState,
};
use crate::render::context::state::{
    AppBackgroundState, CommittedViewState, DrawAssemblyState, FeedbackState, FramePacingMode,
    QualityStats, RenderContext, SlotInteractionGate, Stage0Metrics, StreamingConfig,
    StreamingRuntimeInit, StreamingRuntimeState, WindowedPlacement,
};
use crate::render::gpu::GpuState;
use crate::spatial::navigation::{ViewRuntimeConfig, ViewportState};

pub(super) struct BuildContextInput {
    pub window: Arc<Window>,
    pub gpu: GpuState,
    pub graphics_backend_preference: Option<GraphicsBackendPreference>,
    pub tuning: InitTuning,
    pub cache_resources: CacheInitResources,
    pub bootstrap: BootstrapInitState,
    pub render_resources: RenderInitResources,
}

pub(super) fn build_context(input: BuildContextInput) -> RenderContext {
    let BuildContextInput {
        window,
        gpu,
        graphics_backend_preference,
        tuning,
        cache_resources,
        bootstrap,
        render_resources,
    } = input;

    let CacheInitResources {
        cache_cfg,
        vram_info,
        view_resources,
        texture_systems,
        cache_uniform,
        cache_uniform_buffer,
        bindings,
        gpu_feedback,
    } = cache_resources;

    let RenderInitResources {
        render_pipeline,
        slot_backdrop_pipeline,
        mut backdrop_blur,
        wallpaper_ui,
        wallpaper_preview_ui,
        window_chrome,
        sidebar_ui,
        canvas_context_menu,
        codec_notice,
        slot_backdrop_capacity,
        slot_backdrop_buffer,
        visible_capacity,
        visible_buffer,
        feedback_instance_capacity,
        feedback_instance_buffer,
        feedback_instance_bind_group,
        feedback_collect_buf_bind_group,
    } = render_resources;

    let BootstrapInitState {
        scene,
        slot_paths,
        loader,
    } = bootstrap;

    let (scene_color_texture, scene_color_view) =
        RenderContext::create_scene_color_target(&gpu.device, gpu.config.format, gpu.size);
    let backdrop_blur_size = RenderContext::compute_backdrop_blur_size(gpu.size);
    let blur_fmt = crate::render::ui::backdrop::BLUR_TEXTURE_FORMAT;
    let (backdrop_blur_a_texture, backdrop_blur_a_view) =
        RenderContext::create_scene_color_target(&gpu.device, blur_fmt, backdrop_blur_size);
    let (backdrop_blur_b_texture, backdrop_blur_b_view) =
        RenderContext::create_scene_color_target(&gpu.device, blur_fmt, backdrop_blur_size);
    backdrop_blur.rebuild_bind_groups(
        &gpu.device,
        &scene_color_view,
        &backdrop_blur_a_view,
        &backdrop_blur_b_view,
    );

    // State snapshots.
    let last_epoch_zoom = view_resources.view.zoom;
    let preview_coverage = config::preview_coverage_window(
        cache_cfg.max_thumb_requests_per_frame,
        tuning.min_visible_previews_per_frame,
    );
    let preview_coverage_outstanding_min = preview_coverage.min_outstanding;
    let preview_coverage_outstanding_max = preview_coverage.max_outstanding;
    let preview_coverage_upload_ema = preview_coverage.upload_ema_seed;
    log::info!(
        "Preview coverage window: outstanding min={} max={}",
        preview_coverage_outstanding_min,
        preview_coverage_outstanding_max,
    );
    crate::core::loader::mem_cache::clear_ram_media_slot_assets();
    let document_mode = CanvasDocumentMode::empty();
    let surface_size = gpu.size;
    let windowed_placement = Some(WindowedPlacement {
        position: window.outer_position().ok(),
        size: window.inner_size(),
    });
    RenderContext {
        window,
        windowed_placement,
        window_fake_maximized: false,
        window_was_maximized_before_fullscreen: false,
        window_was_fake_maximized_before_fullscreen: false,
        graphics_backend_preference,
        gpu,
        document: ActiveDocumentState {
            scene,
            slot_paths,
            media_paths: Vec::new(),
            document_mode,
            document_revision: 1,
            background: DocumentBackgroundState {
                auto_frame_pending: true,
                ..DocumentBackgroundState::default()
            },
        },
        viewport: ViewportState::new(
            surface_size,
            view_resources.view,
            ViewRuntimeConfig::new(
                tuning.zoom_reset_settle_frames,
                tuning.zoom_reset_cooldown_frames,
                tuning.preview_soft_reset_pan_delta_px,
                tuning.preview_soft_reset_cooldown_frames,
            ),
            std::time::Instant::now(),
        ),
        render_pipeline,
        slot_backdrop_pipeline,
        scene_color_texture,
        scene_color_view,
        backdrop_blur_a_texture,
        backdrop_blur_a_view,
        backdrop_blur_b_texture,
        backdrop_blur_b_view,
        backdrop_blur_size,

        camera_uniform: view_resources.uniform,
        camera_buffer: view_resources.buffer,
        camera_bind_group_layout: bindings.camera_layout,
        camera_bind_group: bindings.camera_group,

        texture_bind_group_layout: bindings.texture_layout,
        diffuse_bind_group: bindings.diffuse_group,

        cache_uniform,
        cache_uniform_buffer,

        atlas: texture_systems.atlas,
        tile_cache: texture_systems.tile_cache,
        page_table: texture_systems.page_table,
        page_directory: texture_systems.page_directory,
        debug_slot_backdrop_enabled: false,
        committed_view: CommittedViewState::with_visible_capacity(visible_capacity),
        draw_assembly: DrawAssemblyState::new(
            slot_backdrop_capacity,
            slot_backdrop_buffer,
            visible_capacity,
            visible_buffer,
            feedback_instance_capacity,
            feedback_instance_buffer,
            feedback_instance_bind_group,
            feedback_collect_buf_bind_group,
        ),
        hovered_id: None,
        selected_id: None,
        pending_canvas_click: None,
        last_empty_slot_click: None,
        last_media_click: None,

        loader,
        cursor_pos: None,
        keyboard_modifiers: winit::keyboard::ModifiersState::empty(),
        mouse_left_down: false,
        pending_titlebar_drag_origin: None,
        active_client_resize: None,
        frame_pacing_mode: FramePacingMode::VSync,
        frame_count: 0,
        last_update_at: None,
        pending_redraw: false,
        next_continuous_redraw_at: None,
        tabs: Vec::new(),
        active_tab: 0,
        next_tab_id: 1,
        streaming_runtime: StreamingRuntimeState::new(StreamingRuntimeInit {
            last_epoch_zoom,
            preview_coverage_upload_ema,
            preview_coverage_outstanding_min,
            preview_coverage_outstanding_max,
            slot_residency_update_interval_frames: 6,
            slot_residency_grace_frames_idle: 24,
            slot_residency_grace_frames_moving: 12,
            slot_interaction_gate: SlotInteractionGate::new(
                tuning.slot_interaction_off_visible,
                tuning.slot_interaction_on_immediate_visible,
                tuning.slot_interaction_on_delay_frames,
            ),
        }),
        quality_visible_since: HashMap::new(),
        quality_last_cleanup_frame: 0,
        streaming: StreamingConfig {
            prefetch_radius_tiles: cache_cfg.prefetch_radius_tiles,
            max_canvas_media_slot_requests_per_frame: cache_cfg
                .max_canvas_media_slot_requests_per_frame,
            canvas_media_slot_cpu_budget_ms: tuning.canvas_media_slot_cpu_budget_ms,
            max_inflight_canvas_media_slots: tuning.max_inflight_canvas_media_slots,
            max_thumb_requests_per_frame: cache_cfg.max_thumb_requests_per_frame,
            min_visible_previews_per_frame: tuning.min_visible_previews_per_frame,
            min_visible_previews_moving_per_frame: tuning.min_visible_previews_moving_per_frame,
            max_preview_requests_moving_per_frame: tuning.max_preview_requests_moving_per_frame,
            max_canvas_media_slot_queue_len: 100_000,
            cpu_budget_ms_upload: 2,
            max_uploads_per_frame: 200,
            min_visible_canvas_media_slots_per_frame: 32,
            use_gpu_feedback: tuning.use_gpu_feedback,
        },

        last_vram_info: vram_info,
        last_vram_budget_check_at: None,

        quality_stats: QualityStats::default(),
        stage0_metrics: Stage0Metrics::default(),

        gpu_feedback,

        feedback: FeedbackState::default(),
        app_background: AppBackgroundState::default(),

        backdrop_blur,
        wallpaper_ui,
        wallpaper_preview_ui,
        wallpaper_library: crate::core::wallpaper::WallpaperLibrary::load_library(),
        recent_wallpapers: Vec::new(),
        window_chrome,
        sidebar_ui,
        canvas_context_menu,
        codec_notice,
    }
}
