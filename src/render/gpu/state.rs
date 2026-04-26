use crate::render::cache::TileFormat;
use winit::dpi::PhysicalSize;

pub struct GpuState {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: PhysicalSize<u32>,
    pub feedback_rt_supported: bool,
    pub tile_format: TileFormat,
}

impl GpuState {
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn set_present_mode(&mut self, present_mode: wgpu::PresentMode) {
        if self.config.present_mode == present_mode {
            return;
        }
        self.config.present_mode = present_mode;
        if self.config.width > 0 && self.config.height > 0 {
            self.surface.configure(&self.device, &self.config);
        }
        log::info!(
            "Reconfigured surface present mode: {:?}",
            self.config.present_mode
        );
    }
}
