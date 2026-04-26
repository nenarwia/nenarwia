use winit::dpi::PhysicalSize;

use crate::render::ui::{UiVertex, WallpaperUi};

impl WallpaperUi {
    pub fn update_layout(&mut self, queue: &wgpu::Queue, surface_size: PhysicalSize<u32>) {
        if !self.enabled {
            return;
        }
        if surface_size.width == 0 || surface_size.height == 0 {
            return;
        }
        if self.image_width == 0 || self.image_height == 0 {
            return;
        }
        if self.last_surface_width == surface_size.width
            && self.last_surface_height == surface_size.height
        {
            return;
        }

        let surface_aspect = surface_size.width as f32 / surface_size.height as f32;
        let image_aspect = self.image_width as f32 / self.image_height as f32;

        // Cover mode: preserve aspect, center-crop overflow.
        let (u0, u1, v0, v1) = if surface_aspect > image_aspect {
            let visible_v = (image_aspect / surface_aspect).clamp(0.0, 1.0);
            let pad = (1.0 - visible_v) * 0.5;
            (0.0, 1.0, pad, 1.0 - pad)
        } else {
            let visible_u = (surface_aspect / image_aspect).clamp(0.0, 1.0);
            let pad = (1.0 - visible_u) * 0.5;
            (pad, 1.0 - pad, 0.0, 1.0)
        };

        let verts = [
            UiVertex {
                position: [-1.0, -1.0],
                uv: [u0, v1],
            },
            UiVertex {
                position: [1.0, -1.0],
                uv: [u1, v1],
            },
            UiVertex {
                position: [1.0, 1.0],
                uv: [u1, v0],
            },
            UiVertex {
                position: [-1.0, -1.0],
                uv: [u0, v1],
            },
            UiVertex {
                position: [1.0, 1.0],
                uv: [u1, v0],
            },
            UiVertex {
                position: [-1.0, 1.0],
                uv: [u0, v0],
            },
        ];
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&verts));
        self.last_surface_width = surface_size.width;
        self.last_surface_height = surface_size.height;
    }
}
