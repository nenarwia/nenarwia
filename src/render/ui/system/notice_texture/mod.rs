mod codec;
mod geometry;
mod gpu;
mod paint;
mod text_layout;

use crate::core::color::MissingCodecKind;

use winit::dpi::PhysicalSize;

pub(super) struct NoticeTexture {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub close_rect: [u32; 4],
}

pub(super) fn build_notice_texture(
    kinds: &[MissingCodecKind],
    max_width: u32,
) -> Option<NoticeTexture> {
    codec::build_notice_texture(kinds, max_width)
}

pub(super) fn point_in_rect(x: f32, y: f32, rect: [f32; 4]) -> bool {
    geometry::point_in_rect(x, y, rect)
}

pub(super) fn layout_max_width(surface_size: PhysicalSize<u32>) -> u32 {
    geometry::layout_max_width(surface_size)
}

pub(super) fn create_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    gpu::create_texture(device, width, height)
}
