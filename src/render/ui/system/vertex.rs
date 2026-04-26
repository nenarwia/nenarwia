#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct UiVertex {
    pub(crate) position: [f32; 2],
    pub(crate) uv: [f32; 2],
}
