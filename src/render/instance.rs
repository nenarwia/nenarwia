#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
pub struct InstanceRaw {
    /// [x, y, w, h]
    pub data: [f32; 4],
    /// [r, g, b, a]
    pub color: [f32; 4],
    /// [u, v, uw, vh] - Atlas UV
    pub uv_region: [f32; 4],

    /// Streaming params (DESIRED LOD): [pt_x, pt_y, tiles_x, tiles_y]
    /// pt_x/pt_y: origin in PageDirectory texture. pt_x < 0 => no tile mode.
    pub params: [f32; 4],

    /// Streaming params (COARSE fallback LOD): [pt_x, pt_y, tiles_x, tiles_y]
    /// pt_x < 0 => no coarse fallback.
    pub params2: [f32; 4],

    /// Sampling flags: [desired_mode, coarse_mode, atlas_mode, _]
    /// mode: 0 = linear, 1 = mitchell, 2 = catmull-rom, 3 = nearest (pixel-perfect)
    pub sample_flags: [f32; 4],

    /// Local slot UV rect for contain-fit media: [u0, v0, us, vs]
    pub fit_rect: [f32; 4],
}

impl InstanceRaw {
    pub const FULL_SLOT_FIT_RECT: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

    pub fn contain_fit_rect(orig_w: u32, orig_h: u32) -> [f32; 4] {
        if orig_w == 0 || orig_h == 0 {
            return Self::FULL_SLOT_FIT_RECT;
        }

        let aspect = orig_w as f32 / orig_h as f32;
        if !aspect.is_finite() || aspect <= 0.0 {
            return Self::FULL_SLOT_FIT_RECT;
        }

        let (fill_w, fill_h) = if aspect >= 1.0 {
            (1.0, (1.0 / aspect).clamp(0.0, 1.0))
        } else {
            (aspect.clamp(0.0, 1.0), 1.0)
        };
        let pad_x = (1.0 - fill_w) * 0.5;
        let pad_y = (1.0 - fill_h) * 0.5;

        [pad_x, pad_y, fill_w, fill_h]
    }

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 48,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 64,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 80,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 96,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
