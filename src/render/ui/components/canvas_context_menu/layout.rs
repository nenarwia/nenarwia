use winit::dpi::PhysicalSize;

use super::super::{UiUpdatable, UiUpdateCtx};
use super::style::{build_menu_texture, menu_height_px, menu_width_px, write_rect_vertices};
use super::CanvasContextMenuUi;

impl CanvasContextMenuUi {
    pub fn blur_rect_px(&self) -> Option<[f32; 4]> {
        self.open.then_some(self.panel_rect_px)
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_size: PhysicalSize<u32>,
    ) {
        if surface_size.width == 0 || surface_size.height == 0 {
            return;
        }

        if self.texture_dirty
            || self.tex_width != menu_width_px()
            || self.tex_height != menu_height_px()
        {
            let texture = build_menu_texture(
                self.hovered_show_in_explorer,
                self.hovered_delete,
                self.busy,
            );
            self.show_in_explorer_rect_local = [
                texture.show_in_explorer_rect[0] as f32,
                texture.show_in_explorer_rect[1] as f32,
                texture.show_in_explorer_rect[2] as f32,
                texture.show_in_explorer_rect[3] as f32,
            ];
            self.update_texture(device, queue, &texture);
            self.delete_rect_local = [
                texture.delete_rect[0] as f32,
                texture.delete_rect[1] as f32,
                texture.delete_rect[2] as f32,
                texture.delete_rect[3] as f32,
            ];
            self.texture_dirty = false;
        }

        if self.open {
            self.panel_rect_px[2] = self.tex_width as f32;
            self.panel_rect_px[3] = self.tex_height as f32;
            self.panel_rect_px[0] = self.panel_rect_px[0].clamp(
                0.0,
                surface_size.width.saturating_sub(self.tex_width) as f32,
            );
            self.panel_rect_px[1] = self.panel_rect_px[1].clamp(
                0.0,
                surface_size.height.saturating_sub(self.tex_height) as f32,
            );
            write_rect_vertices(queue, &self.vertex_buffer, surface_size, self.panel_rect_px);
        }
    }

    pub(super) fn clamp_to_surface(&mut self, surface_size: PhysicalSize<u32>, x: f32, y: f32) {
        let width = menu_width_px() as f32;
        let height = menu_height_px() as f32;
        self.panel_rect_px = [
            x.clamp(
                0.0,
                surface_size.width.saturating_sub(menu_width_px()) as f32,
            ),
            y.clamp(
                0.0,
                surface_size.height.saturating_sub(menu_height_px()) as f32,
            ),
            width,
            height,
        ];
    }
}

impl UiUpdatable for CanvasContextMenuUi {
    fn update_ui(&mut self, ctx: UiUpdateCtx<'_>) {
        self.update(ctx.device, ctx.queue, ctx.surface_size);
    }
}
