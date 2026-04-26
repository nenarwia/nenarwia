use winit::dpi::PhysicalSize;

use super::super::{UiUpdatable, UiUpdateCtx, UiVertex, CHROME_HEIGHT_PX};
use super::style::build_chrome_texture;
use super::WindowChromeUi;

impl WindowChromeUi {
    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_size: PhysicalSize<u32>,
        maximized: bool,
    ) {
        if surface_size.width == 0 || surface_size.height == 0 {
            return;
        }

        let needs_texture_update = self.last_surface_width != surface_size.width
            || self.last_maximized != maximized
            || self.tex_height != CHROME_HEIGHT_PX
            || self.texture_dirty;
        if needs_texture_update {
            let texture = build_chrome_texture(
                surface_size.width,
                maximized,
                &self.tabs,
                self.active_tab,
                self.hovered_tab,
                self.hovered_close_tab,
                self.hovered_add_tab,
            );
            self.update_texture(device, queue, &texture);
            self.close_rect_px = Some([
                texture.close_rect[0] as f32,
                texture.close_rect[1] as f32,
                texture.close_rect[2] as f32,
                texture.close_rect[3] as f32,
            ]);
            self.minimize_rect_px = Some([
                texture.minimize_rect[0] as f32,
                texture.minimize_rect[1] as f32,
                texture.minimize_rect[2] as f32,
                texture.minimize_rect[3] as f32,
            ]);
            self.maximize_rect_px = Some([
                texture.maximize_rect[0] as f32,
                texture.maximize_rect[1] as f32,
                texture.maximize_rect[2] as f32,
                texture.maximize_rect[3] as f32,
            ]);
            self.tab_indices = texture.tab_indices;
            self.tab_rects_px = texture
                .tab_rects
                .iter()
                .map(|rect| {
                    [
                        rect[0] as f32,
                        rect[1] as f32,
                        rect[2] as f32,
                        rect[3] as f32,
                    ]
                })
                .collect();
            self.tab_close_rects_px = texture
                .tab_close_rects
                .iter()
                .map(|rect| {
                    rect.map(|rect| {
                        [
                            rect[0] as f32,
                            rect[1] as f32,
                            rect[2] as f32,
                            rect[3] as f32,
                        ]
                    })
                })
                .collect();
            self.add_tab_rect_px = texture.add_tab_rect.map(|rect| {
                [
                    rect[0] as f32,
                    rect[1] as f32,
                    rect[2] as f32,
                    rect[3] as f32,
                ]
            });
            self.drag_rect_px = Some([
                texture.drag_rect[0] as f32,
                texture.drag_rect[1] as f32,
                texture.drag_rect[2] as f32,
                texture.drag_rect[3] as f32,
            ]);
            self.last_surface_width = surface_size.width;
            self.last_maximized = maximized;
            self.texture_dirty = false;
        }

        if self.last_surface_height != surface_size.height {
            self.last_surface_height = surface_size.height;
        }
        self.update_vertices(queue, surface_size);
    }

    fn update_vertices(&mut self, queue: &wgpu::Queue, surface_size: PhysicalSize<u32>) {
        if surface_size.width == 0 || surface_size.height == 0 {
            return;
        }

        let box_w = self.tex_width.min(surface_size.width) as f32;
        let box_h = self.tex_height as f32;
        let x = 0.0f32;
        let y = 0.0f32;
        self.window_rect_px = Some([x, y, box_w, box_h]);

        let left = x / surface_size.width as f32 * 2.0 - 1.0;
        let right = (x + box_w) / surface_size.width as f32 * 2.0 - 1.0;
        let top = 1.0 - (y / surface_size.height as f32 * 2.0);
        let bottom = 1.0 - ((y + box_h) / surface_size.height as f32 * 2.0);
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
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&verts));
    }
}

impl UiUpdatable for WindowChromeUi {
    fn update_ui(&mut self, ctx: UiUpdateCtx<'_>) {
        self.update(
            ctx.device,
            ctx.queue,
            ctx.surface_size,
            ctx.window_maximized,
        );
    }
}
