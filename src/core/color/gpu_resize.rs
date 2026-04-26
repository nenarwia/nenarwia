use std::sync::mpsc;
use std::sync::{Mutex, Once, OnceLock};

use wgpu::util::DeviceExt;

const SHADER_SRC: &str = r#"
struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VsOut {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(3.0, 1.0),
    );
    var uv = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 2.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(2.0, 0.0),
    );

    var out: VsOut;
    out.pos = vec4<f32>(pos[vid], 0.0, 1.0);
    out.uv = uv[vid];
    return out;
}

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(src_tex, src_sampler, in.uv);
}
"#;

#[derive(Clone, Copy, Debug)]
enum GpuResizeState {
    Unknown,
    Disabled,
    Ready,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GpuResizeMode {
    Auto,
    On,
    Off,
}

struct GpuResizeContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

static MODE: OnceLock<GpuResizeMode> = OnceLock::new();
static GPU_STATE: OnceLock<Mutex<GpuResizeState>> = OnceLock::new();
static GPU_CTX: OnceLock<Mutex<Option<GpuResizeContext>>> = OnceLock::new();
static GPU_MIN_PIXELS: OnceLock<u64> = OnceLock::new();
static GPU_PREWARM_ENABLED: OnceLock<bool> = OnceLock::new();
static GPU_PREWARM_ONCE: Once = Once::new();

fn gpu_resize_mode() -> GpuResizeMode {
    *MODE.get_or_init(|| {
        let val = std::env::var("CANVAS_GPU_TILE_RESIZE")
            .unwrap_or_default()
            .trim()
            .to_lowercase();
        if val.is_empty() || matches!(val.as_str(), "auto") {
            if cfg!(target_os = "windows") {
                return GpuResizeMode::Auto;
            }
            return GpuResizeMode::Off;
        }
        if matches!(val.as_str(), "0" | "false" | "no" | "off") {
            return GpuResizeMode::Off;
        }
        if matches!(val.as_str(), "1" | "true" | "yes" | "on") {
            return GpuResizeMode::On;
        }
        if cfg!(target_os = "windows") {
            GpuResizeMode::Auto
        } else {
            GpuResizeMode::Off
        }
    })
}

fn state_cell() -> &'static Mutex<GpuResizeState> {
    GPU_STATE.get_or_init(|| Mutex::new(GpuResizeState::Unknown))
}

fn context_cell() -> &'static Mutex<Option<GpuResizeContext>> {
    GPU_CTX.get_or_init(|| Mutex::new(None))
}

fn ensure_context() -> bool {
    if matches!(gpu_resize_mode(), GpuResizeMode::Off) {
        return false;
    }

    if let Ok(state) = state_cell().lock() {
        if matches!(*state, GpuResizeState::Disabled) {
            return false;
        }
        if matches!(*state, GpuResizeState::Ready) {
            return true;
        }
    }

    let created = create_context();
    if let Ok(mut ctx_lock) = context_cell().lock() {
        *ctx_lock = created;
    }
    if let Ok(mut state) = state_cell().lock() {
        let next = if context_cell()
            .lock()
            .ok()
            .and_then(|ctx| ctx.as_ref().map(|_| ()))
            .is_some()
        {
            GpuResizeState::Ready
        } else {
            GpuResizeState::Disabled
        };
        if matches!(next, GpuResizeState::Disabled) && !matches!(*state, GpuResizeState::Disabled) {
            log::warn!(
                "GPU resize: initialization failed, disabling GPU path and using CPU fallback."
            );
        }
        *state = next;
    }
    context_cell()
        .lock()
        .ok()
        .and_then(|ctx| ctx.as_ref().map(|_| ()))
        .is_some()
}

fn create_context() -> Option<GpuResizeContext> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .ok()?;

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("gpu-resize-device"),
        required_features: wgpu::Features::empty(),
        required_limits: adapter.limits(),
        ..Default::default()
    }))
    .ok()?;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("gpu-resize-shader"),
        source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("gpu-resize-bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
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
        label: Some("gpu-resize-pipeline-layout"),
        bind_group_layouts: &[&bind_group_layout],
        immediate_size: 0,
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("gpu-resize-pipeline"),
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
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
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
        label: Some("gpu-resize-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        ..Default::default()
    });

    Some(GpuResizeContext {
        device,
        queue,
        pipeline,
        bind_group_layout,
        sampler,
    })
}

fn align_up(value: u32, align: u32) -> u32 {
    if align == 0 {
        return value;
    }
    value.div_ceil(align) * align
}

fn gpu_resize_min_pixels() -> u64 {
    *GPU_MIN_PIXELS.get_or_init(|| {
        std::env::var("CANVAS_GPU_RESIZE_MIN_PIXELS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .map(|v| v.clamp(0, 16 * 1024 * 1024))
            .unwrap_or(512 * 512)
    })
}

fn gpu_resize_prewarm_enabled() -> bool {
    *GPU_PREWARM_ENABLED.get_or_init(|| {
        let val = std::env::var("CANVAS_GPU_RESIZE_PREWARM")
            .unwrap_or_else(|_| "on".to_string())
            .trim()
            .to_ascii_lowercase();
        matches!(val.as_str(), "1" | "true" | "yes" | "on")
    })
}

pub fn prewarm_gpu_resize_backend() {
    if !gpu_resize_prewarm_enabled() {
        return;
    }

    GPU_PREWARM_ONCE.call_once(|| {
        if matches!(gpu_resize_mode(), GpuResizeMode::Off) {
            return;
        }

        let warm_side = 512u32;
        let warm_src = vec![
            0u8;
            (warm_side as usize)
                .saturating_mul(warm_side as usize)
                .saturating_mul(4)
        ];
        let _ = resize_rgba8_srgb_gpu(&warm_src, warm_side, warm_side, 1, 1);
        let _ = ensure_context();
    });
}

pub fn gpu_resize_enabled_value() -> bool {
    if matches!(gpu_resize_mode(), GpuResizeMode::Off) {
        return false;
    }
    if let Ok(state) = state_cell().lock() {
        return !matches!(*state, GpuResizeState::Disabled);
    }
    true
}

pub fn gpu_resize_should_use(src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> bool {
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return false;
    }
    if !gpu_resize_enabled_value() {
        return false;
    }

    let src_pixels = (src_w as u64).saturating_mul(src_h as u64);
    let dst_pixels = (dst_w as u64).saturating_mul(dst_h as u64);
    src_pixels.max(dst_pixels) >= gpu_resize_min_pixels()
}

pub fn resize_rgba8_srgb_gpu(
    src: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
) -> Option<Vec<u8>> {
    if !gpu_resize_should_use(src_w, src_h, dst_w, dst_h) {
        return None;
    }
    let src_row_bytes = src_w.saturating_mul(4) as usize;
    let src_expected = src_row_bytes.saturating_mul(src_h as usize);
    if src.len() < src_expected {
        return None;
    }
    if !ensure_context() {
        return None;
    }

    let mut ctx_guard = context_cell().lock().ok()?;
    let ctx = ctx_guard.as_mut()?;
    let max_dim = ctx.device.limits().max_texture_dimension_2d;
    if src_w > max_dim || src_h > max_dim || dst_w > max_dim || dst_h > max_dim {
        return None;
    }

    let src_texture = ctx.device.create_texture_with_data(
        &ctx.queue,
        &wgpu::TextureDescriptor {
            label: Some("gpu-resize-src"),
            size: wgpu::Extent3d {
                width: src_w,
                height: src_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        src,
    );
    let src_view = src_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let dst_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gpu-resize-dst"),
        size: wgpu::Extent3d {
            width: dst_w,
            height: dst_h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let dst_view = dst_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("gpu-resize-bg"),
        layout: &ctx.bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&src_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&ctx.sampler),
            },
        ],
    });

    let row_unpadded = dst_w.saturating_mul(4);
    let row_padded = align_up(row_unpadded, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let readback_size = (row_padded as u64).saturating_mul(dst_h as u64);
    let readback = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gpu-resize-readback"),
        size: readback_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("gpu-resize-encoder"),
        });

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("gpu-resize-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &dst_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            multiview_mask: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&ctx.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &dst_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row_padded),
                rows_per_image: Some(dst_h),
            },
        },
        wgpu::Extent3d {
            width: dst_w,
            height: dst_h,
            depth_or_array_layers: 1,
        },
    );

    ctx.queue.submit(Some(encoder.finish()));

    let buffer_slice = readback.slice(..);
    let (tx, rx) = mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
        let _ = tx.send(res.is_ok());
    });
    let _ = ctx.device.poll(wgpu::PollType::wait_indefinitely());
    if rx.recv().ok()? == false {
        return None;
    }

    let data = buffer_slice.get_mapped_range();
    let mut out = vec![0u8; (row_unpadded as usize).saturating_mul(dst_h as usize)];
    for y in 0..dst_h as usize {
        let src_off = y.saturating_mul(row_padded as usize);
        let dst_off = y.saturating_mul(row_unpadded as usize);
        let src_end = src_off.saturating_add(row_unpadded as usize);
        let dst_end = dst_off.saturating_add(row_unpadded as usize);
        if src_end <= data.len() && dst_end <= out.len() {
            out[dst_off..dst_end].copy_from_slice(&data[src_off..src_end]);
        }
    }
    drop(data);
    readback.unmap();

    Some(out)
}
