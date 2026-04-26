use crate::render::ui::WallpaperPreviewUi;

use super::pixel_ops::blit_cover_bilinear;
use super::state::WallpaperPreviewParams;
use super::style::PREVIEW_RECT_LOCAL;
use super::texture_upload::write_texture_region;

impl WallpaperPreviewUi {
    pub(super) fn clear_preview_textures(&mut self, queue: &wgpu::Queue) {
        self.preview_source_ready = false;
        self.preview_blur_ready = false;
        self.preview_source_pixels.clear();
        self.preview_blur_pixels.clear();
        self.write_preview_params(queue);
    }

    pub(super) fn upload_preview_textures(&mut self, queue: &wgpu::Queue) -> Result<(), String> {
        if self.original_width == 0 || self.original_height == 0 || self.original_pixels.is_empty()
        {
            self.clear_preview_textures(queue);
            return Ok(());
        }

        let width = PREVIEW_RECT_LOCAL[2];
        let height = PREVIEW_RECT_LOCAL[3];
        let mut source_cache = vec![0u8; (width as usize) * (height as usize) * 4];
        blit_cover_bilinear(
            &mut source_cache,
            width,
            height,
            [0, 0, width, height],
            self.original_pixels.as_slice(),
            self.original_width,
            self.original_height,
        );
        write_texture_region(
            queue,
            &self.preview_source_texture,
            [0, 0, width, height],
            &source_cache,
            width,
            height,
        );
        // Keep the blur texture initialized even before the async blur result arrives.
        write_texture_region(
            queue,
            &self.preview_blur_texture,
            [0, 0, width, height],
            &source_cache,
            width,
            height,
        );
        self.preview_source_pixels = source_cache;
        self.preview_source_ready = true;
        self.preview_blur_ready = false;

        if self.blurred_pixels.is_some() {
            self.upload_preview_blur_texture(queue)?;
        } else {
            self.preview_blur_pixels.clear();
        }
        self.write_preview_params(queue);
        Ok(())
    }

    pub(super) fn upload_preview_blur_texture(
        &mut self,
        queue: &wgpu::Queue,
    ) -> Result<(), String> {
        let width = PREVIEW_RECT_LOCAL[2];
        let height = PREVIEW_RECT_LOCAL[3];
        let Some(blurred) = self.blurred_pixels.as_ref() else {
            self.preview_blur_ready = false;
            self.preview_blur_pixels.clear();
            self.write_preview_params(queue);
            return Ok(());
        };

        if self.blurred_width == 0 || self.blurred_height == 0 {
            return Err("Wallpaper preview blur has invalid dimensions.".to_string());
        }

        let mut blur_cache = vec![0u8; (width as usize) * (height as usize) * 4];
        blit_cover_bilinear(
            &mut blur_cache,
            width,
            height,
            [0, 0, width, height],
            blurred.as_slice(),
            self.blurred_width,
            self.blurred_height,
        );
        write_texture_region(
            queue,
            &self.preview_blur_texture,
            [0, 0, width, height],
            &blur_cache,
            width,
            height,
        );
        self.preview_blur_pixels = blur_cache;
        self.preview_blur_ready = true;
        self.write_preview_params(queue);
        Ok(())
    }

    pub(super) fn write_preview_params(&self, queue: &wgpu::Queue) {
        let blur_mix = if self.preview_blur_ready {
            self.blur_mix.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let params = WallpaperPreviewParams {
            values: [self.dim_amount.clamp(0.0, 1.0), blur_mix, 0.0, 0.0],
        };
        queue.write_buffer(&self.preview_params_buffer, 0, bytemuck::bytes_of(&params));
    }
}
