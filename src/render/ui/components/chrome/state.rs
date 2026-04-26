use super::super::bind_groups::create_texture_sampler_bind_group;
use super::super::notice_texture::create_texture;
use super::super::UiVertex;

#[derive(Clone)]
pub struct ChromeTabView {
    pub title: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ChromePressTarget {
    CloseWindow,
    MinimizeWindow,
    ToggleWindowMaximize,
    NewTab,
    CloseTab(usize),
}

pub(super) struct ChromeTexture {
    pub(super) pixels: Vec<u8>,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) close_rect: [u32; 4],
    pub(super) minimize_rect: [u32; 4],
    pub(super) maximize_rect: [u32; 4],
    pub(super) tab_indices: Vec<usize>,
    pub(super) tab_rects: Vec<[u32; 4]>,
    pub(super) tab_close_rects: Vec<Option<[u32; 4]>>,
    pub(super) add_tab_rect: Option<[u32; 4]>,
    pub(super) drag_rect: [u32; 4],
}

pub struct WindowChromeUi {
    pub(super) pipeline: wgpu::RenderPipeline,
    pub(super) bind_group_layout: wgpu::BindGroupLayout,
    pub(super) bind_group: wgpu::BindGroup,
    pub(super) sampler: wgpu::Sampler,
    pub(super) texture: wgpu::Texture,
    pub(super) texture_view: wgpu::TextureView,
    pub(super) tex_width: u32,
    pub(super) tex_height: u32,
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) vertex_count: u32,

    pub(super) window_rect_px: Option<[f32; 4]>,
    pub(super) close_rect_px: Option<[f32; 4]>,
    pub(super) minimize_rect_px: Option<[f32; 4]>,
    pub(super) maximize_rect_px: Option<[f32; 4]>,
    pub(super) tab_indices: Vec<usize>,
    pub(super) tab_rects_px: Vec<[f32; 4]>,
    pub(super) tab_close_rects_px: Vec<Option<[f32; 4]>>,
    pub(super) add_tab_rect_px: Option<[f32; 4]>,
    pub(super) drag_rect_px: Option<[f32; 4]>,
    pub(super) last_surface_width: u32,
    pub(super) last_surface_height: u32,
    pub(super) last_maximized: bool,
    pub(super) texture_dirty: bool,
    pub(super) tabs: Vec<ChromeTabView>,
    pub(super) active_tab: usize,
    pub(super) hovered_tab: Option<usize>,
    pub(super) hovered_close_tab: Option<usize>,
    pub(super) hovered_add_tab: bool,
    pub(super) pressed_target: Option<ChromePressTarget>,
}

impl WindowChromeUi {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Window Chrome Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../../ui.wgsl").into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("window_chrome_bind_group_layout"),
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
            label: Some("window_chrome_pipeline_layout"),
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
            label: Some("window_chrome_pipeline"),
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
            label: Some("window_chrome_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let (texture, texture_view) = create_texture(device, 1, 1);
        let bind_group = create_texture_sampler_bind_group(
            device,
            Some("window_chrome_bind_group"),
            &bind_group_layout,
            &texture_view,
            &sampler,
        );
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("window_chrome_vertex_buffer"),
            size: (std::mem::size_of::<UiVertex>() * 6) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            pipeline,
            bind_group_layout,
            bind_group,
            sampler,
            texture,
            texture_view,
            tex_width: 1,
            tex_height: 1,
            vertex_buffer,
            vertex_count: 6,
            window_rect_px: None,
            close_rect_px: None,
            minimize_rect_px: None,
            maximize_rect_px: None,
            tab_indices: Vec::new(),
            tab_rects_px: Vec::new(),
            tab_close_rects_px: Vec::new(),
            add_tab_rect_px: None,
            drag_rect_px: None,
            last_surface_width: 0,
            last_surface_height: 0,
            last_maximized: false,
            texture_dirty: true,
            tabs: Vec::new(),
            active_tab: 0,
            hovered_tab: None,
            hovered_close_tab: None,
            hovered_add_tab: false,
            pressed_target: None,
        }
    }

    pub fn clear_hover_state(&mut self) -> bool {
        self.set_hover_state(None, None, false)
    }

    pub fn clear_interaction_state(&mut self) -> bool {
        let hover_changed = self.clear_hover_state();
        let pressed_changed = self.clear_pressed_target();
        hover_changed || pressed_changed
    }

    pub(super) fn clear_pressed_target(&mut self) -> bool {
        self.set_pressed_target(None)
    }

    pub(super) fn set_pressed_target(&mut self, pressed_target: Option<ChromePressTarget>) -> bool {
        if self.pressed_target == pressed_target {
            return false;
        }
        self.pressed_target = pressed_target;
        true
    }

    pub(super) fn set_hover_state(
        &mut self,
        hovered_tab: Option<usize>,
        hovered_close_tab: Option<usize>,
        hovered_add_tab: bool,
    ) -> bool {
        if self.hovered_tab == hovered_tab
            && self.hovered_close_tab == hovered_close_tab
            && self.hovered_add_tab == hovered_add_tab
        {
            return false;
        }
        self.hovered_tab = hovered_tab;
        self.hovered_close_tab = hovered_close_tab;
        self.hovered_add_tab = hovered_add_tab;
        self.texture_dirty = true;
        true
    }

    pub fn sync_tabs(&mut self, tabs: &[ChromeTabView], active_tab: usize) -> bool {
        let active_tab = if tabs.is_empty() {
            0
        } else {
            active_tab.min(tabs.len().saturating_sub(1))
        };
        if self.tabs.len() == tabs.len()
            && self.active_tab == active_tab
            && self
                .tabs
                .iter()
                .zip(tabs.iter())
                .all(|(lhs, rhs)| lhs.title == rhs.title)
        {
            return false;
        }
        self.tabs = tabs.to_vec();
        self.active_tab = active_tab;
        if self.hovered_tab.is_some_and(|idx| idx >= self.tabs.len()) {
            self.hovered_tab = None;
        }
        if self
            .hovered_close_tab
            .is_some_and(|idx| idx >= self.tabs.len())
        {
            self.hovered_close_tab = None;
        }
        self.texture_dirty = true;
        true
    }
}
