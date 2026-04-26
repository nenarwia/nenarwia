use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;

use winit::dpi::PhysicalSize;

use crate::core::wallpaper::{build_blurred_pixels, wallpaper_blur_max_dim_for_surface};
use crate::render::ui::WallpaperPreviewUi;

impl WallpaperPreviewUi {
    pub fn poll_blur_job(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_size: PhysicalSize<u32>,
    ) {
        if !self.blur_loading {
            return;
        }
        let Some(rx) = self.blur_rx.as_ref() else {
            self.blur_loading = false;
            return;
        };

        match rx.try_recv() {
            Ok(Ok((blurred_pixels, width, height))) => {
                self.blurred_pixels = Some(blurred_pixels);
                self.blurred_width = width;
                self.blurred_height = height;
                self.blur_rx = None;
                self.blur_loading = false;
                if self.visible {
                    if let Err(err) = self.upload_preview_blur_texture(queue) {
                        log::warn!("Failed to upload wallpaper preview blur: {}", err);
                    }
                    if self.blur_enabled {
                        self.start_blur_transition(1.0);
                    }
                    if let Err(err) = self.rebuild_texture(device, queue) {
                        log::warn!("Failed to rebuild wallpaper preview after blur: {}", err);
                    } else {
                        self.update_layout(queue, surface_size);
                    }
                }
            }
            Ok(Err(err)) => {
                log::warn!("Wallpaper blur preparation failed: {}", err);
                self.blur_rx = None;
                self.blur_loading = false;
            }
            Err(TryRecvError::Disconnected) => {
                self.blur_rx = None;
                self.blur_loading = false;
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    pub fn advance_blur_animation(&mut self, queue: &wgpu::Queue) {
        let Some(started_at) = self.blur_anim_started_at else {
            return;
        };

        let duration_s = self.blur_anim_duration.as_secs_f32().max(0.0001);
        let elapsed_s = std::time::Instant::now()
            .saturating_duration_since(started_at)
            .as_secs_f32();
        let t = (elapsed_s / duration_s).clamp(0.0, 1.0);
        let eased = ease_in_out_cubic(t);
        let next_mix = self.blur_anim_from + (self.blur_anim_to - self.blur_anim_from) * eased;
        if (next_mix - self.blur_mix).abs() > 0.001 || t >= 1.0 {
            self.blur_mix = next_mix.clamp(0.0, 1.0);
            self.write_preview_params(queue);
        }
        if t >= 1.0 {
            self.blur_mix = self.blur_anim_to;
            self.blur_anim_started_at = None;
            self.write_preview_params(queue);
        }
    }

    pub(super) fn start_blur_job(&mut self, surface_size: PhysicalSize<u32>) {
        if self.original_width == 0 || self.original_height == 0 {
            return;
        }
        let pixels = self.original_pixels.clone();
        let width = self.original_width;
        let height = self.original_height;
        let blur_max_dim =
            wallpaper_blur_max_dim_for_surface(surface_size.width, surface_size.height);
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = build_blurred_pixels(width, height, pixels.as_slice(), blur_max_dim)
                .map_err(|err| err.to_string());
            let _ = tx.send(result);
        });
        self.blur_rx = Some(rx);
        self.blur_loading = true;
    }
}

fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - ((-2.0 * t + 2.0).powi(3) * 0.5)
    }
}
