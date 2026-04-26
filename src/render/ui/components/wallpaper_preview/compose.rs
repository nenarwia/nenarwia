use ab_glyph::{Font, PxScale, ScaleFont};

use crate::render::ui::WallpaperPreviewUi;

use super::super::bind_groups::create_texture_sampler_bind_group;
use super::super::font::load_font;
use super::super::notice_texture::create_texture;
use super::super::raster::draw_text_line;
use super::dim_control::compose_dim_control_pixels;
use super::pixel_ops::{blit_rgba_region, fill_rect_region};
use super::style::{
    APPLY_RECT_LOCAL, CANCEL_RECT_LOCAL, DIALOG_HEIGHT_PX, DIALOG_WIDTH_PX, DIM_RECT_LOCAL,
    PREVIEW_RECT_LOCAL, TOGGLE_RECT_LOCAL,
};
use super::texture_upload::write_texture_region;

impl WallpaperPreviewUi {
    pub(super) fn rebuild_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(), String> {
        let width = DIALOG_WIDTH_PX;
        let height = DIALOG_HEIGHT_PX;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];

        fill_rect_region(
            &mut pixels,
            width,
            height,
            [0, 0, width, height],
            [18, 18, 18, 236],
        );
        fill_rect_region(
            &mut pixels,
            width,
            height,
            [0, 0, width, 1],
            [255, 255, 255, 28],
        );
        fill_rect_region(
            &mut pixels,
            width,
            height,
            [0, height.saturating_sub(1), width, 1],
            [255, 255, 255, 24],
        );

        fill_rect_region(
            &mut pixels,
            width,
            height,
            PREVIEW_RECT_LOCAL,
            [30, 30, 30, 255],
        );

        let blur_ready = self.blurred_pixels.is_some();
        let blur_waiting = self.blur_enabled && self.blur_loading && !blur_ready;
        let toggle_bg = if blur_waiting {
            [92, 74, 38, 220]
        } else if self.blur_enabled {
            [44, 112, 72, 220]
        } else {
            [66, 66, 66, 210]
        };
        let apply_bg = if blur_waiting {
            [86, 86, 86, 210]
        } else {
            [48, 122, 212, 235]
        };
        fill_rect_region(&mut pixels, width, height, TOGGLE_RECT_LOCAL, toggle_bg);
        let dim_pixels = compose_dim_control_pixels(self.dim_amount);
        blit_rgba_region(
            &mut pixels,
            width,
            height,
            DIM_RECT_LOCAL,
            &dim_pixels,
            DIM_RECT_LOCAL[2],
            DIM_RECT_LOCAL[3],
        );
        fill_rect_region(
            &mut pixels,
            width,
            height,
            CANCEL_RECT_LOCAL,
            [70, 70, 70, 220],
        );
        fill_rect_region(&mut pixels, width, height, APPLY_RECT_LOCAL, apply_bg);

        if let Some(font) = load_font() {
            let title_scale = PxScale::from(16.0);
            let title_scaled = font.as_scaled(title_scale);
            let title_ascent = title_scaled.ascent();
            draw_text_line(super::super::raster::TextLineParams {
                pixels: &mut pixels,
                width,
                height,
                font: &font,
                scale: title_scale,
                x: 20.0,
                y: 18.0 + title_ascent,
                text: "Wallpaper Preview",
                color: [244, 244, 244, 255],
            });

            let item_scale = PxScale::from(14.0);
            let item_scaled = font.as_scaled(item_scale);
            let item_ascent = item_scaled.ascent();

            if self.source_loading {
                draw_text_line(super::super::raster::TextLineParams {
                    pixels: &mut pixels,
                    width,
                    height,
                    font: &font,
                    scale: item_scale,
                    x: PREVIEW_RECT_LOCAL[0] as f32 + 14.0,
                    y: PREVIEW_RECT_LOCAL[1] as f32 + 20.0 + item_ascent,
                    text: "Loading image...",
                    color: [220, 220, 220, 210],
                });
            }

            draw_text_line(super::super::raster::TextLineParams {
                pixels: &mut pixels,
                width,
                height,
                font: &font,
                scale: item_scale,
                x: TOGGLE_RECT_LOCAL[0] as f32 + 10.0,
                y: TOGGLE_RECT_LOCAL[1] as f32
                    + ((TOGGLE_RECT_LOCAL[3] as f32 - 14.0) * 0.5)
                    + item_ascent,
                text: if blur_waiting {
                    "Blur wallpaper: Loading..."
                } else if self.blur_enabled {
                    "Blur wallpaper: ON"
                } else {
                    "Blur wallpaper: OFF"
                },
                color: [255, 255, 255, 245],
            });

            draw_text_line(super::super::raster::TextLineParams {
                pixels: &mut pixels,
                width,
                height,
                font: &font,
                scale: item_scale,
                x: CANCEL_RECT_LOCAL[0] as f32 + 22.0,
                y: CANCEL_RECT_LOCAL[1] as f32
                    + ((CANCEL_RECT_LOCAL[3] as f32 - 14.0) * 0.5)
                    + item_ascent,
                text: "Cancel",
                color: [244, 244, 244, 245],
            });
            draw_text_line(super::super::raster::TextLineParams {
                pixels: &mut pixels,
                width,
                height,
                font: &font,
                scale: item_scale,
                x: APPLY_RECT_LOCAL[0] as f32 + 28.0,
                y: APPLY_RECT_LOCAL[1] as f32
                    + ((APPLY_RECT_LOCAL[3] as f32 - 14.0) * 0.5)
                    + item_ascent,
                text: "Apply",
                color: [250, 250, 250, 255],
            });
        }

        self.write_preview_params(queue);
        if self.tex_width != width || self.tex_height != height {
            let (texture, texture_view) = create_texture(device, width, height);
            self.texture = texture;
            self.texture_view = texture_view;
            self.tex_width = width;
            self.tex_height = height;
            self.bind_group = create_texture_sampler_bind_group(
                device,
                Some("wallpaper_preview_bind_group"),
                &self.bind_group_layout,
                &self.texture_view,
                &self.sampler,
            );
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        Ok(())
    }

    pub(super) fn update_dim_texture(&mut self, queue: &wgpu::Queue) -> Result<(), String> {
        if self.tex_width != DIALOG_WIDTH_PX || self.tex_height != DIALOG_HEIGHT_PX {
            return Err(
                "Wallpaper preview texture is not ready for partial dim update.".to_string(),
            );
        }

        self.write_preview_params(queue);
        let dim_pixels = compose_dim_control_pixels(self.dim_amount);
        write_texture_region(
            queue,
            &self.texture,
            DIM_RECT_LOCAL,
            &dim_pixels,
            DIM_RECT_LOCAL[2],
            DIM_RECT_LOCAL[3],
        );
        Ok(())
    }
}
