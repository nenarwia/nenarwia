use super::super::notice_texture::create_texture;
use super::super::UiVertex;

pub(super) struct MenuTexture {
    pub(super) pixels: Vec<u8>,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) show_in_explorer_rect: [u32; 4],
    pub(super) delete_rect: [u32; 4],
}

pub struct CanvasContextMenuUi {
    pub(super) pipeline: wgpu::RenderPipeline,
    pub(super) bind_group_layout: wgpu::BindGroupLayout,
    pub(super) sampler: wgpu::Sampler,

    pub(super) texture: wgpu::Texture,
    pub(super) texture_view: wgpu::TextureView,
    pub(super) bind_group: wgpu::BindGroup,
    pub(super) tex_width: u32,
    pub(super) tex_height: u32,
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) vertex_count: u32,

    pub(super) panel_rect_px: [f32; 4],
    pub(super) show_in_explorer_rect_local: [f32; 4],
    pub(super) delete_rect_local: [f32; 4],
    pub(super) hovered_show_in_explorer: bool,
    pub(super) hovered_delete: bool,
    pub(super) texture_dirty: bool,
    pub(super) open: bool,
    pub(super) busy: bool,
    pub(super) target_slot_id: Option<u64>,
}

impl CanvasContextMenuUi {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("canvas_context_menu_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../../ui.wgsl").into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("canvas_context_menu_bind_group_layout"),
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
            label: Some("canvas_context_menu_pipeline_layout"),
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
            label: Some("canvas_context_menu_pipeline"),
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
            label: Some("canvas_context_menu_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let (texture, texture_view) = create_texture(device, 1, 1);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("canvas_context_menu_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("canvas_context_menu_vertex_buffer"),
            size: (std::mem::size_of::<UiVertex>() * 6) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            bind_group_layout,
            sampler,
            texture,
            texture_view,
            bind_group,
            tex_width: 1,
            tex_height: 1,
            vertex_buffer,
            vertex_count: 6,
            panel_rect_px: [0.0, 0.0, 0.0, 0.0],
            show_in_explorer_rect_local: [0.0; 4],
            delete_rect_local: [0.0; 4],
            hovered_show_in_explorer: false,
            hovered_delete: false,
            texture_dirty: true,
            open: false,
            busy: false,
            target_slot_id: None,
        }
    }
}
