use winit::dpi::PhysicalSize;

use crate::render::ui::WallpaperPreviewUi;

use super::style::{
    APPLY_RECT_LOCAL, CANCEL_RECT_LOCAL, DIM_RECT_LOCAL, DIM_TRACK_LEFT_PAD_PX,
    DIM_TRACK_RIGHT_PAD_PX, PREVIEW_RECT_LOCAL, TOGGLE_RECT_LOCAL,
};

impl WallpaperPreviewUi {
    pub fn update_layout(&mut self, queue: &wgpu::Queue, surface_size: PhysicalSize<u32>) {
        if !self.visible || surface_size.width == 0 || surface_size.height == 0 {
            return;
        }
        if self.last_surface_width == surface_size.width
            && self.last_surface_height == surface_size.height
        {
            return;
        }
        self.last_surface_width = surface_size.width;
        self.last_surface_height = surface_size.height;

        let x = ((surface_size.width.saturating_sub(self.tex_width)) as f32 * 0.5).max(8.0);
        let y = ((surface_size.height.saturating_sub(self.tex_height)) as f32 * 0.5).max(8.0);
        self.dialog_rect_px = [x, y, self.tex_width as f32, self.tex_height as f32];
        self.toggle_rect_px = [
            x + TOGGLE_RECT_LOCAL[0] as f32,
            y + TOGGLE_RECT_LOCAL[1] as f32,
            TOGGLE_RECT_LOCAL[2] as f32,
            TOGGLE_RECT_LOCAL[3] as f32,
        ];
        self.dim_rect_px = [
            x + DIM_RECT_LOCAL[0] as f32,
            y + DIM_RECT_LOCAL[1] as f32,
            DIM_RECT_LOCAL[2] as f32,
            DIM_RECT_LOCAL[3] as f32,
        ];
        self.dim_track_x0_px = x + (DIM_RECT_LOCAL[0] + DIM_TRACK_LEFT_PAD_PX) as f32;
        self.dim_track_x1_px =
            x + (DIM_RECT_LOCAL[0] + DIM_RECT_LOCAL[2] - DIM_TRACK_RIGHT_PAD_PX) as f32;
        self.cancel_rect_px = [
            x + CANCEL_RECT_LOCAL[0] as f32,
            y + CANCEL_RECT_LOCAL[1] as f32,
            CANCEL_RECT_LOCAL[2] as f32,
            CANCEL_RECT_LOCAL[3] as f32,
        ];
        self.apply_rect_px = [
            x + APPLY_RECT_LOCAL[0] as f32,
            y + APPLY_RECT_LOCAL[1] as f32,
            APPLY_RECT_LOCAL[2] as f32,
            APPLY_RECT_LOCAL[3] as f32,
        ];

        super::super::geometry::write_rect_vertices(
            queue,
            &self.vertex_buffer,
            surface_size,
            [x, y, self.tex_width as f32, self.tex_height as f32],
        );
        super::super::geometry::write_rect_vertices(
            queue,
            &self.preview_vertex_buffer,
            surface_size,
            [
                x + PREVIEW_RECT_LOCAL[0] as f32,
                y + PREVIEW_RECT_LOCAL[1] as f32,
                PREVIEW_RECT_LOCAL[2] as f32,
                PREVIEW_RECT_LOCAL[3] as f32,
            ],
        );
    }
}
