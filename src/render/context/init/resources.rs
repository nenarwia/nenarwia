use crate::core::vram::{self, VramInfo};
use crate::render::cache::CacheUniform;
use crate::render::context::budget::{self, CacheConfig};
use crate::render::context::factory::{bindings, resources as factory_resources, textures};
use crate::render::gpu::GpuState;
use crate::render::instance::InstanceRaw;
use crate::render::pipeline::PipelineFactory;
use crate::render::streaming::feedback::{FeedbackInstance, GpuFeedback};
use crate::render::ui::{
    BackdropBlurUi, CanvasContextMenuUi, CodecNoticeUi, SidebarUi, WallpaperPreviewUi, WallpaperUi,
    WindowChromeUi,
};

pub(super) struct CacheInitResources {
    pub cache_cfg: CacheConfig,
    pub vram_info: Option<VramInfo>,
    pub view_resources: factory_resources::ViewResources,
    pub texture_systems: textures::TextureSystems,
    pub cache_uniform: CacheUniform,
    pub cache_uniform_buffer: wgpu::Buffer,
    pub bindings: bindings::Bindings,
    pub gpu_feedback: Option<GpuFeedback>,
}

pub(super) struct RenderInitResources {
    pub render_pipeline: wgpu::RenderPipeline,
    pub slot_backdrop_pipeline: wgpu::RenderPipeline,

    pub backdrop_blur: BackdropBlurUi,
    pub wallpaper_ui: WallpaperUi,
    pub wallpaper_preview_ui: WallpaperPreviewUi,
    pub window_chrome: WindowChromeUi,
    pub sidebar_ui: SidebarUi,
    pub canvas_context_menu: CanvasContextMenuUi,
    pub codec_notice: CodecNoticeUi,

    pub slot_backdrop_capacity: usize,
    pub slot_backdrop_buffer: wgpu::Buffer,

    pub visible_capacity: usize,
    pub visible_buffer: wgpu::Buffer,

    pub feedback_instance_capacity: usize,
    pub feedback_instance_buffer: wgpu::Buffer,
    pub feedback_instance_bind_group: wgpu::BindGroup,
    pub feedback_collect_buf_bind_group: wgpu::BindGroup,
}

pub(super) fn create_cache_init_resources(gpu: &GpuState, max_dim: u32) -> CacheInitResources {
    let vram_info = vram::query_vram();
    if let Some(v) = vram_info {
        log::info!(
            "VRAM: total={:.2}GiB used={:.2}GiB free={:.2}GiB",
            v.total_gib(),
            v.used_bytes as f32 / (1024.0 * 1024.0 * 1024.0),
            v.free_gib()
        );
    } else {
        log::warn!("VRAM probe unavailable. Falling back to safe defaults.");
    }

    let cache_cfg = budget::decide_cache_config(vram_info, max_dim);

    // Create camera and uniform resources.
    let view_resources =
        factory_resources::create_view(gpu.config.width, gpu.config.height, &gpu.device);

    // Create texture-backed cache systems.
    let texture_systems = textures::create_texture_systems(
        &gpu.device,
        &gpu.queue,
        &cache_cfg,
        max_dim,
        gpu.tile_format,
    );

    // Create cache uniforms.
    let (cache_uniform, cache_uniform_buffer) =
        factory_resources::create_cache_uniform(texture_systems.tile_cache.cols, &gpu.device);

    // Build bind groups.
    let bindings = bindings::create_bind_groups(bindings::CreateBindingsInput {
        device: &gpu.device,
        camera_buffer: &view_resources.buffer,
        atlas_views: texture_systems.atlas.views(),
        atlas_sampler_linear: texture_systems.atlas.sampler_linear(),
        atlas_sampler_nearest: texture_systems.atlas.sampler_nearest(),
        tile_cache_view: &texture_systems.tile_cache.view,
        page_dir_view: &texture_systems.page_directory.view,
        cache_uniform_buffer: &cache_uniform_buffer,
    });

    let gpu_feedback = Some(GpuFeedback::new(
        &gpu.device,
        &bindings.camera_layout,
        gpu.feedback_rt_supported,
    ));

    CacheInitResources {
        cache_cfg,
        vram_info,
        view_resources,
        texture_systems,
        cache_uniform,
        cache_uniform_buffer,
        bindings,
        gpu_feedback,
    }
}

pub(super) fn create_render_init_resources(
    gpu: &GpuState,
    camera_layout: &wgpu::BindGroupLayout,
    texture_layout: &wgpu::BindGroupLayout,
    gpu_feedback: Option<&GpuFeedback>,
) -> RenderInitResources {
    // Build render pipeline and UI resources.
    let render_pipeline =
        PipelineFactory::create_render_pipeline(gpu, camera_layout, texture_layout);
    let slot_backdrop_pipeline = PipelineFactory::create_slot_backdrop_pipeline(gpu, camera_layout);

    let backdrop_blur = BackdropBlurUi::new(&gpu.device, gpu.config.format);
    let wallpaper_ui = WallpaperUi::new(&gpu.device, gpu.config.format);
    let wallpaper_preview_ui = WallpaperPreviewUi::new(&gpu.device, gpu.config.format);
    let window_chrome = WindowChromeUi::new(&gpu.device, gpu.config.format);
    let sidebar_ui = SidebarUi::new(&gpu.device, gpu.config.format);
    let canvas_context_menu = CanvasContextMenuUi::new(&gpu.device, gpu.config.format);
    let codec_notice = CodecNoticeUi::new(&gpu.device, gpu.config.format);

    let visible_capacity = 1024usize;
    let slot_backdrop_capacity = 1usize;
    let slot_backdrop_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Slot Backdrop Buffer"),
        size: (slot_backdrop_capacity * std::mem::size_of::<InstanceRaw>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let visible_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Visible Instances Buffer"),
        size: (visible_capacity * std::mem::size_of::<InstanceRaw>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let feedback_instance_capacity = visible_capacity;
    let feedback_instance_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Feedback Instance Buffer"),
        size: (feedback_instance_capacity * std::mem::size_of::<FeedbackInstance>()) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let feedback_instance_bind_group = gpu_feedback
        .as_ref()
        .map(|fb| fb.create_instance_bind_group(&gpu.device, &feedback_instance_buffer))
        .unwrap();
    let feedback_collect_buf_bind_group = gpu_feedback
        .as_ref()
        .map(|fb| fb.create_collect_buf_bind_group(&gpu.device, &feedback_instance_buffer))
        .unwrap();

    RenderInitResources {
        render_pipeline,
        slot_backdrop_pipeline,

        backdrop_blur,
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
    }
}
