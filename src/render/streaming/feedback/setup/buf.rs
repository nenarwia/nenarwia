pub(super) fn create_collect_buf_resources(
    device: &wgpu::Device,
) -> (wgpu::BindGroupLayout, wgpu::ComputePipeline) {
    let collect_buf_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Feedback Collect BUF BGL"),
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
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let collect_buf_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Feedback Collect BUF Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../../feedback_collect_buf.wgsl").into()),
    });

    let collect_buf_layout_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Feedback Collect BUF Pipeline Layout"),
        bind_group_layouts: &[&collect_buf_layout],
        immediate_size: 0,
    });

    let collect_buf_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Feedback Collect BUF Pipeline"),
        layout: Some(&collect_buf_layout_pl),
        module: &collect_buf_shader,
        entry_point: Some("cs_main"),
        compilation_options: Default::default(),
        cache: None,
    });

    (collect_buf_layout, collect_buf_pipeline)
}
