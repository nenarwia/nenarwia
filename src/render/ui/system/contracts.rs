use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::keyboard::ModifiersState;

use crate::core::app_settings::GraphicsBackendPreference;

use super::UiAction;

// Shared UI "contract": all interactive UI pieces can be updated and clicked the same way.
#[derive(Clone, Copy)]
pub(crate) struct UiUpdateCtx<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub surface_size: PhysicalSize<u32>,
    pub window_maximized: bool,
    pub vsync_enabled: bool,
    pub graphics_backend_preference: Option<GraphicsBackendPreference>,
    pub debug_slot_backdrop_enabled: bool,
}

pub(crate) trait UiUpdatable {
    fn update_ui(&mut self, ctx: UiUpdateCtx<'_>);
}

pub(crate) trait UiClickable {
    fn on_click(
        &mut self,
        pos: PhysicalPosition<f64>,
        modifiers: ModifiersState,
    ) -> Option<UiAction>;
}

pub(crate) trait UiRenderable {
    fn render_under_chrome(&self, _encoder: &mut wgpu::CommandEncoder, _view: &wgpu::TextureView) {}

    fn render_overlay(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView);
}
