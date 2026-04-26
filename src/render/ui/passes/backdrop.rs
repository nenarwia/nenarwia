use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

use super::backdrop_state::{BackdropBlurBindGroups, BackdropParams, MAX_OVERLAY_BLUR_RECTS};
use super::{BackdropBlurUi, CHROME_HEIGHT_PX};

/// Linear format used for blur intermediate textures to avoid sRGB gamma
/// round-trips that corrupt colors across multiple blur passes.
pub const BLUR_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

fn create_params_buffer(
    device: &wgpu::Device,
    label: &str,
    params: &BackdropParams,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::bytes_of(params),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    })
}

fn write_params_if_changed(
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    current: &mut BackdropParams,
    next: BackdropParams,
) {
    if *current == next {
        return;
    }
    queue.write_buffer(buffer, 0, bytemuck::bytes_of(&next));
    *current = next;
}

impl BackdropBlurUi {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("backdrop_blur_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../backdrop_blur.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("backdrop_blur_bind_group_layout"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("backdrop_blur_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        // Blur pipeline targets Rgba16Float intermediate textures (linear space).
        let blur_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("backdrop_blur_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: BLUR_TEXTURE_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // Composite pipeline targets the swapchain (sRGB surface format).
        let composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("backdrop_blur_composite_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
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
            label: Some("backdrop_blur_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Each pass type gets its own uniform buffer so that queue.write_buffer
        // calls don't overwrite each other (all writes are flushed before the
        // first render pass in a submit).
        let dummy = BackdropParams::initial();
        let downsample_params_buf =
            create_params_buffer(device, "backdrop_downsample_params", &dummy);
        let blur_h_params_buf = create_params_buffer(device, "backdrop_blur_h_params", &dummy);
        let blur_v_params_buf = create_params_buffer(device, "backdrop_blur_v_params", &dummy);
        let composite_params_buf =
            create_params_buffer(device, "backdrop_composite_params", &dummy);

        Self {
            blur_pipeline,
            composite_pipeline,
            bind_group_layout,
            sampler,
            downsample_params_buf,
            blur_h_params_buf,
            blur_v_params_buf,
            composite_params_buf,
            bind_groups: None,
            downsample_params: dummy,
            blur_h_params: dummy,
            blur_v_params: dummy,
            composite_params: dummy,
        }
    }

    pub fn rebuild_bind_groups(
        &mut self,
        device: &wgpu::Device,
        scene_view: &wgpu::TextureView,
        blur_a_view: &wgpu::TextureView,
        blur_b_view: &wgpu::TextureView,
    ) {
        let downsample = self.create_pass_bind_group(
            device,
            "backdrop_blur_bg_downsample",
            scene_view,
            scene_view,
            &self.downsample_params_buf,
        );
        let blur_h = self.create_pass_bind_group(
            device,
            "backdrop_blur_bg_h",
            blur_a_view,
            blur_a_view,
            &self.blur_h_params_buf,
        );
        let blur_v = self.create_pass_bind_group(
            device,
            "backdrop_blur_bg_v",
            blur_b_view,
            blur_b_view,
            &self.blur_v_params_buf,
        );
        let composite = self.create_pass_bind_group(
            device,
            "backdrop_blur_bg_composite",
            blur_a_view,
            scene_view,
            &self.composite_params_buf,
        );
        self.bind_groups = Some(BackdropBlurBindGroups {
            downsample,
            blur_h,
            blur_v,
            composite,
        });
    }

    fn create_pass_bind_group(
        &self,
        device: &wgpu::Device,
        label: &str,
        source_view: &wgpu::TextureView,
        scene_base_view: &wgpu::TextureView,
        params_buf: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(scene_base_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buf.as_entire_binding(),
                },
            ],
        })
    }

    pub fn render(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        blur_a_view: &wgpu::TextureView,
        blur_b_view: &wgpu::TextureView,
        surface_size: PhysicalSize<u32>,
        blur_size: PhysicalSize<u32>,
        overlay_blur_rects: &[[f32; 4]],
    ) {
        if surface_size.width == 0
            || surface_size.height == 0
            || blur_size.width == 0
            || blur_size.height == 0
        {
            return;
        }

        let empty_blur_rects = [[0.0; 4]; MAX_OVERLAY_BLUR_RECTS];
        let mut extra_blur_rects = [[0.0; 4]; MAX_OVERLAY_BLUR_RECTS];
        let mut extra_blur_rect_count = 0u32;
        for rect in overlay_blur_rects
            .iter()
            .copied()
            .filter(|rect| rect[2] > 0.5 && rect[3] > 0.5)
            .take(MAX_OVERLAY_BLUR_RECTS)
        {
            extra_blur_rects[extra_blur_rect_count as usize] = rect;
            extra_blur_rect_count = extra_blur_rect_count.saturating_add(1);
        }

        // Only the composite pass needs overlay rects; keeping the first
        // three pass params stable avoids uniform churn during panel motion.
        let downsample_params = BackdropParams {
            source_size: [surface_size.width as f32, surface_size.height as f32],
            target_size: [blur_size.width as f32, blur_size.height as f32],
            surface_size: [surface_size.width as f32, surface_size.height as f32],
            blur_axis: [0.0, 0.0],
            chrome_height_px: CHROME_HEIGHT_PX as f32,
            pass_kind: 0,
            extra_blur_rect_count: 0,
            saturate: 1.4,
            extra_blur_rects: empty_blur_rects,
        };
        let blur_h_params = BackdropParams {
            source_size: [blur_size.width as f32, blur_size.height as f32],
            target_size: [blur_size.width as f32, blur_size.height as f32],
            surface_size: [surface_size.width as f32, surface_size.height as f32],
            blur_axis: [1.0, 0.0],
            chrome_height_px: CHROME_HEIGHT_PX as f32,
            pass_kind: 1,
            extra_blur_rect_count: 0,
            saturate: 1.4,
            extra_blur_rects: empty_blur_rects,
        };
        let blur_v_params = BackdropParams {
            blur_axis: [0.0, 1.0],
            ..blur_h_params
        };
        let composite_params = BackdropParams {
            source_size: [blur_size.width as f32, blur_size.height as f32],
            target_size: [surface_size.width as f32, surface_size.height as f32],
            surface_size: [surface_size.width as f32, surface_size.height as f32],
            blur_axis: [0.0, 0.0],
            chrome_height_px: CHROME_HEIGHT_PX as f32,
            pass_kind: 2,
            extra_blur_rect_count,
            saturate: 1.4,
            extra_blur_rects,
        };

        write_params_if_changed(
            queue,
            &self.downsample_params_buf,
            &mut self.downsample_params,
            downsample_params,
        );
        write_params_if_changed(
            queue,
            &self.blur_h_params_buf,
            &mut self.blur_h_params,
            blur_h_params,
        );
        write_params_if_changed(
            queue,
            &self.blur_v_params_buf,
            &mut self.blur_v_params,
            blur_v_params,
        );
        write_params_if_changed(
            queue,
            &self.composite_params_buf,
            &mut self.composite_params,
            composite_params,
        );

        let Some(bind_groups) = self.bind_groups.as_ref() else {
            log::warn!("Backdrop blur bind groups are missing.");
            return;
        };

        // 0) Downsample scene -> blur A (quarter res, linear float).
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Backdrop Blur Downsample"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: blur_a_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                multiview_mask: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.blur_pipeline);
            pass.set_bind_group(0, &bind_groups.downsample, &[]);
            pass.draw(0..6, 0..1);
        }

        // Single round of H+V separable Gaussian blur.
        // Horizontal: A -> B.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Backdrop Blur H"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: blur_b_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                multiview_mask: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.blur_pipeline);
            pass.set_bind_group(0, &bind_groups.blur_h, &[]);
            pass.draw(0..6, 0..1);
        }

        // Vertical: B -> A.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Backdrop Blur V"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: blur_a_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                multiview_mask: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.blur_pipeline);
            pass.set_bind_group(0, &bind_groups.blur_v, &[]);
            pass.draw(0..6, 0..1);
        }

        // Composite: blurred titlebar + original scene -> swapchain.
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Backdrop Blur Composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            multiview_mask: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.composite_pipeline);
        pass.set_bind_group(0, &bind_groups.composite, &[]);
        pass.draw(0..6, 0..1);
    }
}
