use crate::render::ui::{UiRenderable, UiVertex, WallpaperPreviewUi};
use wgpu::util::DeviceExt;

use super::super::bind_groups::create_texture_sampler_bind_group;
use super::super::notice_texture::create_texture;
use super::state::WallpaperPreviewParams;
use super::style::PREVIEW_RECT_LOCAL;

impl WallpaperPreviewUi {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wallpaper_preview_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../../ui.wgsl").into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wallpaper_preview_bind_group_layout"),
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
            label: Some("wallpaper_preview_pipeline_layout"),
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
            label: Some("wallpaper_preview_pipeline"),
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
        let preview_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wallpaper_preview_image_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("preview.wgsl").into()),
        });
        let preview_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("wallpaper_preview_image_bind_group_layout"),
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
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let preview_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("wallpaper_preview_image_pipeline_layout"),
                bind_group_layouts: &[&preview_bind_group_layout],
                immediate_size: 0,
            });
        let preview_vertex_layout = wgpu::VertexBufferLayout {
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
        let preview_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wallpaper_preview_image_pipeline"),
            layout: Some(&preview_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &preview_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[preview_vertex_layout],
            },
            fragment: Some(wgpu::FragmentState {
                module: &preview_shader,
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
            label: Some("wallpaper_preview_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let (texture, texture_view) = create_texture(device, 1, 1);
        let (preview_source_texture, preview_source_texture_view) =
            create_texture(device, PREVIEW_RECT_LOCAL[2], PREVIEW_RECT_LOCAL[3]);
        let (preview_blur_texture, preview_blur_texture_view) =
            create_texture(device, PREVIEW_RECT_LOCAL[2], PREVIEW_RECT_LOCAL[3]);
        let preview_params = WallpaperPreviewParams {
            values: [0.0, 0.0, 0.0, 0.0],
        };
        let preview_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("wallpaper_preview_image_params_buffer"),
            contents: bytemuck::bytes_of(&preview_params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = create_texture_sampler_bind_group(
            device,
            Some("wallpaper_preview_bind_group"),
            &bind_group_layout,
            &texture_view,
            &sampler,
        );
        let preview_bind_group = create_preview_bind_group(
            device,
            &preview_bind_group_layout,
            &preview_source_texture_view,
            &preview_blur_texture_view,
            &sampler,
            &preview_params_buffer,
        );
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wallpaper_preview_vertex_buffer"),
            size: (std::mem::size_of::<UiVertex>() * 6) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let preview_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wallpaper_preview_image_vertex_buffer"),
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
            preview_pipeline,
            preview_source_texture,
            preview_blur_texture,
            preview_bind_group,
            preview_params_buffer,
            preview_vertex_buffer,
            preview_vertex_count: 6,
            preview_source_ready: false,
            preview_blur_ready: false,
            visible: false,
            source_loading: false,
            blur_enabled: false,
            original_pixels: std::sync::Arc::new(Vec::new()),
            original_width: 0,
            original_height: 0,
            blurred_pixels: None,
            blurred_width: 0,
            blurred_height: 0,
            blur_rx: None,
            blur_loading: false,
            blur_mix: 0.0,
            blur_anim_from: 0.0,
            blur_anim_to: 0.0,
            blur_anim_started_at: None,
            blur_anim_duration: std::time::Duration::from_millis(180),
            preview_source_pixels: Vec::new(),
            preview_blur_pixels: Vec::new(),
            dim_amount: 0.0,
            dim_dragging: false,
            editing_wallpaper_id: None,
            selected_source_path: None,
            dialog_rect_px: [0.0; 4],
            toggle_rect_px: [0.0; 4],
            dim_rect_px: [0.0; 4],
            dim_track_x0_px: 0.0,
            dim_track_x1_px: 0.0,
            apply_rect_px: [0.0; 4],
            cancel_rect_px: [0.0; 4],
            last_surface_width: 0,
            last_surface_height: 0,
        }
    }

    pub fn render(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        if !self.visible {
            return;
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Wallpaper Preview Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            multiview_mask: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
        if !self.source_loading && self.preview_source_ready {
            pass.set_pipeline(&self.preview_pipeline);
            pass.set_bind_group(0, &self.preview_bind_group, &[]);
            pass.set_vertex_buffer(0, self.preview_vertex_buffer.slice(..));
            pass.draw(0..self.preview_vertex_count, 0..1);
        }
    }
}

impl UiRenderable for WallpaperPreviewUi {
    fn render_overlay(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        self.render(encoder, view);
    }
}

fn create_preview_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    source_view: &wgpu::TextureView,
    blur_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    params_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("wallpaper_preview_image_bind_group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(source_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(blur_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: params_buffer.as_entire_binding(),
            },
        ],
    })
}
