use winit::dpi::PhysicalSize;

use super::vertex::UiVertex;

pub(crate) fn write_rect_vertices(
    queue: &wgpu::Queue,
    vertex_buffer: &wgpu::Buffer,
    surface_size: PhysicalSize<u32>,
    rect: [f32; 4],
) {
    if surface_size.width == 0 || surface_size.height == 0 {
        return;
    }

    let x = rect[0];
    let y = rect[1];
    let w = rect[2];
    let h = rect[3];

    let left = x / surface_size.width as f32 * 2.0 - 1.0;
    let right = (x + w) / surface_size.width as f32 * 2.0 - 1.0;
    let top = 1.0 - (y / surface_size.height as f32 * 2.0);
    let bottom = 1.0 - ((y + h) / surface_size.height as f32 * 2.0);
    let verts = [
        UiVertex {
            position: [left, bottom],
            uv: [0.0, 1.0],
        },
        UiVertex {
            position: [right, bottom],
            uv: [1.0, 1.0],
        },
        UiVertex {
            position: [right, top],
            uv: [1.0, 0.0],
        },
        UiVertex {
            position: [left, bottom],
            uv: [0.0, 1.0],
        },
        UiVertex {
            position: [right, top],
            uv: [1.0, 0.0],
        },
        UiVertex {
            position: [left, top],
            uv: [0.0, 0.0],
        },
    ];
    queue.write_buffer(vertex_buffer, 0, bytemuck::cast_slice(&verts));
}
