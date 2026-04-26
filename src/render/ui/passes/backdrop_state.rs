use super::CHROME_HEIGHT_PX;

pub(super) const MAX_OVERLAY_BLUR_RECTS: usize = 2;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct BackdropParams {
    pub source_size: [f32; 2],
    pub target_size: [f32; 2],
    pub surface_size: [f32; 2],
    pub blur_axis: [f32; 2],
    pub chrome_height_px: f32,
    pub pass_kind: u32,
    pub extra_blur_rect_count: u32,
    pub saturate: f32,
    pub extra_blur_rects: [[f32; 4]; MAX_OVERLAY_BLUR_RECTS],
}

impl BackdropParams {
    pub(super) fn initial() -> Self {
        Self {
            source_size: [0.0, 0.0],
            target_size: [0.0, 0.0],
            surface_size: [0.0, 0.0],
            blur_axis: [0.0, 0.0],
            chrome_height_px: CHROME_HEIGHT_PX as f32,
            pass_kind: u32::MAX,
            extra_blur_rect_count: 0,
            saturate: 1.4,
            extra_blur_rects: [[0.0; 4]; MAX_OVERLAY_BLUR_RECTS],
        }
    }
}

pub(super) struct BackdropBlurBindGroups {
    pub downsample: wgpu::BindGroup,
    pub blur_h: wgpu::BindGroup,
    pub blur_v: wgpu::BindGroup,
    pub composite: wgpu::BindGroup,
}

pub struct BackdropBlurUi {
    pub(super) blur_pipeline: wgpu::RenderPipeline,
    pub(super) composite_pipeline: wgpu::RenderPipeline,
    pub(super) bind_group_layout: wgpu::BindGroupLayout,
    pub(super) sampler: wgpu::Sampler,
    pub(super) downsample_params_buf: wgpu::Buffer,
    pub(super) blur_h_params_buf: wgpu::Buffer,
    pub(super) blur_v_params_buf: wgpu::Buffer,
    pub(super) composite_params_buf: wgpu::Buffer,
    pub(super) bind_groups: Option<BackdropBlurBindGroups>,
    pub(super) downsample_params: BackdropParams,
    pub(super) blur_h_params: BackdropParams,
    pub(super) blur_v_params: BackdropParams,
    pub(super) composite_params: BackdropParams,
}
