use crate::render::cache::CacheUniform;
use crate::spatial::camera::CameraUniform;
use crate::spatial::view::ViewState;
use wgpu::util::DeviceExt;

pub struct ViewResources {
    pub view: ViewState,
    pub uniform: CameraUniform,
    pub buffer: wgpu::Buffer,
}

pub fn create_view(width: u32, height: u32, device: &wgpu::Device) -> ViewResources {
    let view = ViewState::new(width, height);
    let mut uniform = CameraUniform::new();
    uniform.update_view_proj(&view);

    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Camera Buffer"),
        contents: bytemuck::cast_slice(&[uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    ViewResources {
        view,
        uniform,
        buffer,
    }
}

pub fn create_cache_uniform(cols: u32, device: &wgpu::Device) -> (CacheUniform, wgpu::Buffer) {
    let uniform = CacheUniform::new(cols);
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Cache Uniform Buffer"),
        contents: bytemuck::cast_slice(&[uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    (uniform, buffer)
}
