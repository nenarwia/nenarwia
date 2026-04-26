use crate::render::instance::InstanceRaw;

use super::super::{RtResources, FEEDBACK_TEX_H, FEEDBACK_TEX_W};

pub(super) fn create_rt_resources(
    device: &wgpu::Device,
    camera_layout: &wgpu::BindGroupLayout,
    feedback_instance_layout: &wgpu::BindGroupLayout,
    header: &wgpu::Buffer,
    output: &wgpu::Buffer,
) -> RtResources {
    let feedback_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Feedback TileId Texture"),
        size: wgpu::Extent3d {
            width: FEEDBACK_TEX_W,
            height: FEEDBACK_TEX_H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Uint,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let feedback_view = feedback_tex.create_view(&wgpu::TextureViewDescriptor::default());

    let feedback_valid_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Feedback Valid Texture"),
        size: wgpu::Extent3d {
            width: FEEDBACK_TEX_W,
            height: FEEDBACK_TEX_H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Uint,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let feedback_valid_view =
        feedback_valid_tex.create_view(&wgpu::TextureViewDescriptor::default());

    let feedback_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Feedback Render Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../../feedback_render.wgsl").into()),
    });

    let feedback_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Feedback Render Pipeline Layout"),
        bind_group_layouts: &[camera_layout, feedback_instance_layout],
        immediate_size: 0,
    });

    let feedback_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Feedback Render Pipeline"),
        layout: Some(&feedback_layout),
        vertex: wgpu::VertexState {
            module: &feedback_shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[InstanceRaw::desc()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &feedback_shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[
                Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba32Uint,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                }),
                Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R32Uint,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                }),
            ],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    let collect_rt_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Feedback Collect RT Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../../feedback_collect.wgsl").into()),
    });

    let collect_rt_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Feedback Collect RT BGL"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Uint,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Uint,
                },
                count: None,
            },
        ],
    });

    let collect_rt_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Feedback Collect RT BG"),
        layout: &collect_rt_bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: header.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&feedback_view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&feedback_valid_view),
            },
        ],
    });

    let collect_rt_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Feedback Collect RT Pipeline Layout"),
        bind_group_layouts: &[&collect_rt_bgl],
        immediate_size: 0,
    });

    let collect_rt_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Feedback Collect RT Pipeline"),
        layout: Some(&collect_rt_layout),
        module: &collect_rt_shader,
        entry_point: Some("cs_main"),
        compilation_options: Default::default(),
        cache: None,
    });

    RtResources {
        _feedback_tex: feedback_tex,
        feedback_view,
        _feedback_valid_tex: feedback_valid_tex,
        feedback_valid_view,
        feedback_pipeline,
        collect_rt_pipeline,
        collect_rt_bind_group,
    }
}
