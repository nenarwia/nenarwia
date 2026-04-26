// FILE: src/render/compute/types.rs

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InteractionUniform {
    pub hovered_instance_idx: u32,
    pub loaded_instance_idx: u32,
    pub _pad: [u32; 2],
}
