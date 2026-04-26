use super::super::notice_texture::create_texture;
use super::super::{
    UiVertex, BURGER_BTN_LEFT_PX, BURGER_BTN_SIZE_PX, BURGER_BTN_TOP_PX, SIDEBAR_ANIMATION_MS,
    SIDEBAR_WIDTH_PX,
};
use super::nav::MAX_NAV_ITEMS;
use crate::core::app_settings::GraphicsBackendPreference;
use crate::core::process_memory::ProcessRamUsage;

pub(super) struct PanelTexture {
    pub(super) pixels: Vec<u8>,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) nav_item_rects: [[u32; 4]; MAX_NAV_ITEMS],
    pub(super) debug_slot_backdrop_rect: [u32; 4],
    pub(super) fps_toggle_rect: [u32; 4],
    pub(super) backend_toggle_rect: [u32; 4],
    pub(super) wallpaper_rect: [u32; 4],
    pub(super) recent_wallpaper_rects: Vec<[u32; 4]>,
}

pub(super) struct BurgerTexture {
    pub(super) pixels: Vec<u8>,
    pub(super) width: u32,
    pub(super) height: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SidebarSavedWallpaperItem {
    pub id: u64,
    pub thumb_pixels: Vec<u8>,
    pub thumb_width: u32,
    pub thumb_height: u32,
    pub is_current: bool,
}

const CLEAR_CANVAS_RAM_SAMPLE_INTERVAL_MS: u64 = 250;

pub struct SidebarUi {
    pub(super) pipeline: wgpu::RenderPipeline,
    pub(super) bind_group_layout: wgpu::BindGroupLayout,
    pub(super) sampler: wgpu::Sampler,

    pub(super) panel_texture: wgpu::Texture,
    pub(super) panel_texture_view: wgpu::TextureView,
    pub(super) panel_bind_group: wgpu::BindGroup,
    pub(super) panel_tex_width: u32,
    pub(super) panel_tex_height: u32,
    pub(super) panel_vertex_buffer: wgpu::Buffer,
    pub(super) panel_vertex_count: u32,

    pub(super) burger_texture: wgpu::Texture,
    pub(super) burger_texture_view: wgpu::TextureView,
    pub(super) burger_bind_group: wgpu::BindGroup,
    pub(super) burger_tex_width: u32,
    pub(super) burger_tex_height: u32,
    pub(super) burger_vertex_buffer: wgpu::Buffer,
    pub(super) burger_vertex_count: u32,

    pub(super) panel_rect_px: [f32; 4],
    pub(super) burger_rect_px: [f32; 4],
    pub(super) nav_item_rects_local: [[f32; 4]; MAX_NAV_ITEMS],
    pub(super) debug_slot_backdrop_rect_local: [f32; 4],
    pub(super) fps_toggle_rect_local: [f32; 4],
    pub(super) backend_toggle_rect_local: [f32; 4],
    pub(super) wallpaper_rect_local: [f32; 4],
    pub(super) recent_wallpaper_rects_local: Vec<[f32; 4]>,

    pub(super) open_t: f32,
    pub(super) target_open: bool,
    pub(super) anim_from_t: f32,
    pub(super) anim_to_t: f32,
    pub(super) anim_started_at: Option<std::time::Instant>,
    pub(super) anim_duration: std::time::Duration,
    pub(super) last_surface_height: u32,
    pub(super) burger_texture_ready: bool,
    pub(super) hovered_burger: bool,
    pub(super) hovered_nav_item: Option<usize>,
    pub(super) hovered_debug_slot_backdrop: bool,
    pub(super) hovered_fps_toggle: bool,
    pub(super) hovered_backend_toggle: bool,
    pub(super) hovered_wallpaper: bool,
    pub(super) hovered_recent_wallpaper: Option<usize>,
    pub(super) active_nav_item: Option<usize>,
    pub(super) active_wallpaper: bool,
    pub(super) vsync_enabled: bool,
    pub(super) graphics_backend_preference: Option<GraphicsBackendPreference>,
    pub(super) debug_slot_backdrop_enabled: bool,
    pub(super) clear_canvas_ram_usage: Option<ProcessRamUsage>,
    pub(super) last_clear_canvas_ram_sample_at: Option<std::time::Instant>,
    pub(super) recent_wallpapers: Vec<SidebarSavedWallpaperItem>,
    pub(super) panel_texture_dirty: bool,
    pub(super) burger_texture_dirty: bool,
}

impl SidebarUi {
    pub(super) fn clear_canvas_ram_sample_interval() -> std::time::Duration {
        std::time::Duration::from_millis(CLEAR_CANVAS_RAM_SAMPLE_INTERVAL_MS)
    }

    pub fn set_active_nav_item(&mut self, active_nav_item: Option<usize>) {
        if self.active_nav_item == active_nav_item && !self.active_wallpaper {
            return;
        }
        self.active_nav_item = active_nav_item;
        self.active_wallpaper = false;
        self.panel_texture_dirty = true;
    }

    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sidebar_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../../ui.wgsl").into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sidebar_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sidebar_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<UiVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: std::mem::size_of::<[f32; 2]>() as u64,
                    shader_location: 1,
                },
            ],
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sidebar_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[vertex_layout],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sidebar_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let (panel_texture, panel_texture_view) = create_texture(device, 1, 1);
        let panel_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sidebar_panel_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&panel_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let panel_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sidebar_panel_vertex_buffer"),
            size: (std::mem::size_of::<UiVertex>() * 6) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create a placeholder; real burger texture is uploaded on first update via queue.
        let (burger_texture, burger_texture_view) = create_texture(device, 1, 1);
        let burger_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sidebar_burger_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&burger_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let burger_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sidebar_burger_vertex_buffer"),
            size: (std::mem::size_of::<UiVertex>() * 6) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            bind_group_layout,
            sampler,
            panel_texture,
            panel_texture_view,
            panel_bind_group,
            panel_tex_width: 1,
            panel_tex_height: 1,
            panel_vertex_buffer,
            panel_vertex_count: 6,
            burger_texture,
            burger_texture_view,
            burger_bind_group,
            burger_tex_width: 1,
            burger_tex_height: 1,
            burger_vertex_buffer,
            burger_vertex_count: 6,
            panel_rect_px: [0.0, 0.0, SIDEBAR_WIDTH_PX as f32, 0.0],
            burger_rect_px: [
                BURGER_BTN_LEFT_PX as f32,
                BURGER_BTN_TOP_PX as f32,
                BURGER_BTN_SIZE_PX as f32,
                BURGER_BTN_SIZE_PX as f32,
            ],
            nav_item_rects_local: [[0.0; 4]; MAX_NAV_ITEMS],
            debug_slot_backdrop_rect_local: [0.0; 4],
            fps_toggle_rect_local: [0.0; 4],
            backend_toggle_rect_local: [0.0; 4],
            wallpaper_rect_local: [0.0; 4],
            recent_wallpaper_rects_local: Vec::new(),
            open_t: 1.0,
            target_open: true,
            anim_from_t: 1.0,
            anim_to_t: 1.0,
            anim_started_at: None,
            anim_duration: std::time::Duration::from_millis(SIDEBAR_ANIMATION_MS),
            last_surface_height: 0,
            burger_texture_ready: false,
            hovered_burger: false,
            hovered_nav_item: None,
            hovered_debug_slot_backdrop: false,
            hovered_fps_toggle: false,
            hovered_backend_toggle: false,
            hovered_wallpaper: false,
            hovered_recent_wallpaper: None,
            active_nav_item: None,
            active_wallpaper: false,
            vsync_enabled: true,
            graphics_backend_preference: None,
            debug_slot_backdrop_enabled: false,
            clear_canvas_ram_usage: None,
            last_clear_canvas_ram_sample_at: None,
            recent_wallpapers: Vec::new(),
            panel_texture_dirty: true,
            burger_texture_dirty: true,
        }
    }
}
