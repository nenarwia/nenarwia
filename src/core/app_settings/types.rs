use serde::{Deserialize, Serialize};

const APP_SETTINGS_VERSION: u32 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphicsBackendPreference {
    Vulkan,
    Dx12,
}

impl GraphicsBackendPreference {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Vulkan => "Vulkan",
            Self::Dx12 => "DX12",
        }
    }

    pub const fn toggled(self) -> Self {
        match self {
            Self::Vulkan => Self::Dx12,
            Self::Dx12 => Self::Vulkan,
        }
    }

    pub const fn wgpu_backends(self) -> wgpu::Backends {
        match self {
            Self::Vulkan => wgpu::Backends::VULKAN,
            Self::Dx12 => wgpu::Backends::DX12,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_app_settings_version")]
    pub version: u32,
    #[serde(default = "default_windows_graphics_backend")]
    pub windows_graphics_backend: GraphicsBackendPreference,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            version: APP_SETTINGS_VERSION,
            windows_graphics_backend: default_windows_graphics_backend(),
        }
    }
}

impl AppSettings {
    pub fn sanitized(mut self) -> Self {
        self.version = APP_SETTINGS_VERSION;
        self
    }
}

const fn default_app_settings_version() -> u32 {
    APP_SETTINGS_VERSION
}

pub(super) const fn default_windows_graphics_backend() -> GraphicsBackendPreference {
    GraphicsBackendPreference::Vulkan
}
