use std::path::PathBuf;
use std::sync::Arc;

use winit::dpi::PhysicalSize;

use crate::render::ui::WallpaperPreviewUi;

impl WallpaperPreviewUi {
    pub fn open_loading(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_size: PhysicalSize<u32>,
        selected_source_path: Option<PathBuf>,
        editing_wallpaper_id: Option<u64>,
        dim_amount: f32,
    ) -> Result<(), String> {
        self.visible = true;
        self.source_loading = true;
        self.blur_enabled = false;
        self.blur_rx = None;
        self.blur_loading = false;
        self.original_pixels = Arc::new(Vec::new());
        self.original_width = 0;
        self.original_height = 0;
        self.blurred_pixels = None;
        self.blurred_width = 0;
        self.blurred_height = 0;
        self.blur_mix = 0.0;
        self.blur_anim_from = 0.0;
        self.blur_anim_to = 0.0;
        self.blur_anim_started_at = None;
        self.dim_amount = dim_amount.clamp(0.0, 1.0);
        self.clear_preview_textures(queue);
        self.rebuild_texture(device, queue)?;
        self.update_layout(queue, surface_size);
        self.selected_source_path = selected_source_path;
        self.editing_wallpaper_id = editing_wallpaper_id;
        Ok(())
    }

    pub fn open_from_rgba(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_size: PhysicalSize<u32>,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
        blurred_pixels: Option<Vec<u8>>,
        blurred_width: u32,
        blurred_height: u32,
        selected_source_path: PathBuf,
        editing_wallpaper_id: Option<u64>,
        dim_amount: f32,
        blur_enabled: bool,
    ) -> Result<(), String> {
        if width == 0 || height == 0 {
            return Err("Wallpaper preview image has invalid dimensions.".to_string());
        }
        self.source_loading = false;
        self.original_pixels = Arc::new(pixels);
        self.original_width = width;
        self.original_height = height;
        self.blurred_pixels = blurred_pixels;
        self.blurred_width = blurred_width;
        self.blurred_height = blurred_height;
        self.blur_mix = if blur_enabled && self.blurred_pixels.is_some() {
            1.0
        } else {
            0.0
        };
        self.blur_anim_from = 0.0;
        self.blur_anim_to = 0.0;
        self.blur_anim_started_at = None;
        self.preview_source_pixels.clear();
        self.preview_blur_pixels.clear();
        self.selected_source_path = Some(selected_source_path);
        self.editing_wallpaper_id = editing_wallpaper_id;
        self.dim_amount = dim_amount.clamp(0.0, 1.0);
        self.blur_rx = None;
        self.blur_loading = false;
        if self.blurred_pixels.is_none() {
            self.start_blur_job(surface_size);
        }
        self.blur_enabled = blur_enabled;
        self.visible = true;
        self.upload_preview_textures(queue)?;
        self.rebuild_texture(device, queue)?;
        self.update_layout(queue, surface_size);
        Ok(())
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn toggle_blur(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_size: PhysicalSize<u32>,
    ) -> Result<(), String> {
        if !self.visible {
            return Ok(());
        }
        if self.source_loading {
            return Ok(());
        }
        self.blur_enabled = !self.blur_enabled;
        if self.blur_enabled {
            if self.blurred_pixels.is_some() {
                self.start_blur_transition(1.0);
            } else if !self.blur_loading {
                self.start_blur_job(surface_size);
            }
        } else {
            self.start_blur_transition(0.0);
        }
        self.rebuild_texture(device, queue)?;
        self.update_layout(queue, surface_size);
        Ok(())
    }

    pub fn cancel(&mut self) {
        self.visible = false;
        self.dim_dragging = false;
        self.selected_source_path = None;
        self.editing_wallpaper_id = None;
        self.blur_anim_started_at = None;
    }

    pub fn blur_enabled(&self) -> bool {
        self.blur_enabled
    }

    pub fn dim_amount(&self) -> f32 {
        self.dim_amount
    }

    pub fn editing_wallpaper_id(&self) -> Option<u64> {
        self.editing_wallpaper_id
    }

    pub fn selected_source_path(&self) -> Option<&std::path::Path> {
        self.selected_source_path.as_deref()
    }

    pub fn needs_continuous_redraw(&self) -> bool {
        self.visible && (self.blur_loading || self.blur_anim_started_at.is_some())
    }

    pub(super) fn start_blur_transition(&mut self, target_mix: f32) {
        let target_mix = target_mix.clamp(0.0, 1.0);
        if (self.blur_mix - target_mix).abs() <= 0.001 {
            self.blur_mix = target_mix;
            self.blur_anim_from = target_mix;
            self.blur_anim_to = target_mix;
            self.blur_anim_started_at = None;
            return;
        }
        self.blur_anim_from = self.blur_mix;
        self.blur_anim_to = target_mix;
        self.blur_anim_started_at = Some(std::time::Instant::now());
    }
}
