use winit::dpi::{PhysicalPosition, PhysicalSize};

use crate::render::ui::{UiAction, WallpaperPreviewUi};

use super::super::notice_texture::point_in_rect;

impl WallpaperPreviewUi {
    pub fn handle_mouse_down(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_size: PhysicalSize<u32>,
        pos: PhysicalPosition<f64>,
    ) -> Option<UiAction> {
        if !self.visible {
            return None;
        }
        let x = pos.x as f32;
        let y = pos.y as f32;
        if point_in_rect(x, y, self.toggle_rect_px) {
            if self.source_loading {
                return Some(UiAction::Consume);
            }
            return Some(UiAction::WallpaperPreviewToggleBlur);
        }
        if point_in_rect(x, y, self.dim_rect_px) {
            self.dim_dragging = true;
            self.update_dim_from_cursor(device, queue, surface_size, x);
            return Some(UiAction::Consume);
        }
        if point_in_rect(x, y, self.apply_rect_px) {
            if self.source_loading {
                return Some(UiAction::Consume);
            }
            if self.blur_enabled && self.blur_loading {
                return Some(UiAction::Consume);
            }
            return Some(UiAction::WallpaperPreviewApply);
        }
        if point_in_rect(x, y, self.cancel_rect_px) {
            return Some(UiAction::WallpaperPreviewCancel);
        }
        Some(UiAction::Consume)
    }

    pub fn handle_cursor_moved(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_size: PhysicalSize<u32>,
        pos: PhysicalPosition<f64>,
    ) -> bool {
        if !self.visible || !self.dim_dragging {
            return false;
        }
        self.update_dim_from_cursor(device, queue, surface_size, pos.x as f32);
        true
    }

    pub fn handle_mouse_up(&mut self) -> bool {
        if !self.visible || !self.dim_dragging {
            return false;
        }
        self.dim_dragging = false;
        true
    }

    fn update_dim_from_cursor(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_size: PhysicalSize<u32>,
        cursor_x: f32,
    ) {
        let denom = (self.dim_track_x1_px - self.dim_track_x0_px).max(1.0);
        let next = ((cursor_x - self.dim_track_x0_px) / denom).clamp(0.0, 1.0);
        let next = (next * 100.0).round() / 100.0;
        if (self.dim_amount - next).abs() <= 0.0001 {
            return;
        }
        self.dim_amount = next;
        if let Err(err) = self.update_dim_texture(queue) {
            log::warn!(
                "Failed to update wallpaper preview dim regions: {}; rebuilding full texture",
                err
            );
            if let Err(rebuild_err) = self.rebuild_texture(device, queue) {
                log::warn!(
                    "Failed to rebuild wallpaper preview after dim change: {}",
                    rebuild_err
                );
                return;
            }
            self.update_layout(queue, surface_size);
        }
    }
}
