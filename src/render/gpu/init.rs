use super::state::GpuState;
use crate::core::app_settings;
use crate::render::cache::TileFormat;
use std::sync::Arc;
use winit::window::Window;

fn supports_feedback_rt(adapter: &wgpu::Adapter) -> bool {
    let rgba = adapter.get_texture_format_features(wgpu::TextureFormat::Rgba32Uint);
    let rgba_ok = rgba
        .allowed_usages
        .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
        && rgba
            .allowed_usages
            .contains(wgpu::TextureUsages::TEXTURE_BINDING);
    if !rgba_ok {
        return false;
    }
    let r8 = adapter.get_texture_format_features(wgpu::TextureFormat::R8Uint);
    r8.allowed_usages
        .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
        && r8
            .allowed_usages
            .contains(wgpu::TextureUsages::TEXTURE_BINDING)
}

fn select_tile_format(_adapter: &wgpu::Adapter) -> TileFormat {
    TileFormat::rgba8_srgb()
}

#[cfg(not(target_os = "windows"))]
async fn request_adapter_for_backends(
    window: Arc<Window>,
    backends: wgpu::Backends,
) -> Option<(wgpu::Surface<'static>, wgpu::Adapter)> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends,
        ..Default::default()
    });
    let surface = instance.create_surface(window).ok()?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .ok()?;
    Some((surface, adapter))
}

#[cfg(target_os = "windows")]
async fn request_windows_surface_and_adapter(
    window: Arc<Window>,
) -> (wgpu::Surface<'static>, wgpu::Adapter) {
    let preferred = app_settings::load_windows_graphics_backend_preference();
    log::info!("WGPU backend preference: {}", preferred.label());

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    let surface = instance
        .create_surface(window)
        .expect("Failed to create WGPU surface");

    let adapters = instance.enumerate_adapters(preferred.wgpu_backends()).await;
    for adapter in adapters {
        if !adapter.is_surface_supported(&surface) {
            continue;
        }
        return (surface, adapter);
    }

    let fallback = preferred.toggled();
    log::warn!(
        "Preferred WGPU backend {} unavailable; falling back to {}.",
        preferred.label(),
        fallback.label()
    );

    let fallback_adapters = instance.enumerate_adapters(fallback.wgpu_backends()).await;
    for adapter in fallback_adapters {
        if !adapter.is_surface_supported(&surface) {
            continue;
        }
        return (surface, adapter);
    }

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to find adapter on any backend");
    (surface, adapter)
}

impl GpuState {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        #[cfg(target_os = "windows")]
        let (surface, adapter) = request_windows_surface_and_adapter(window.clone()).await;

        #[cfg(not(target_os = "windows"))]
        let (surface, adapter) =
            request_adapter_for_backends(window.clone(), wgpu::Backends::all())
                .await
                .expect("Failed to find adapter");
        let adapter_info = adapter.get_info();
        log::info!(
            "WGPU adapter: backend={:?} name={} vendor={:#06x} device={:#06x} driver={}",
            adapter_info.backend,
            adapter_info.name,
            adapter_info.vendor,
            adapter_info.device,
            adapter_info.driver
        );

        let feedback_rt_supported = supports_feedback_rt(&adapter);
        if !feedback_rt_supported {
            log::warn!("Feedback RT unsupported on this adapter; using buffer feedback fallback.");
        }

        let tile_format = select_tile_format(&adapter);
        log::info!("Tile cache format: RGBA8");

        let required_features = wgpu::Features::empty();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features,
                ..Default::default()
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        log::info!("Surface present modes: {:?}", surface_caps.present_modes);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo, // VSync enabled
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);
        log::info!("Configured surface present mode: {:?}", config.present_mode);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            feedback_rt_supported,
            tile_format,
        }
    }
}
