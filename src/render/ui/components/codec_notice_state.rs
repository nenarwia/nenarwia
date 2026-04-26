use std::collections::HashSet;

use crate::core::color::MissingCodecKind;

pub struct CodecNoticeUi {
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

    pub(super) visible: bool,
    pub(super) dismissed: bool,
    pub(super) dismissed_kinds: HashSet<MissingCodecKind>,
    pub(super) active_kinds: Vec<MissingCodecKind>,

    pub(super) box_rect_px: Option<[f32; 4]>,
    pub(super) close_rect_px: Option<[f32; 4]>,
    pub(super) close_rect_local_px: [u32; 4],
    pub(super) last_layout_width: u32,
}
