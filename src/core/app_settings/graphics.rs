use anyhow::Result;

use super::persistence::{load_app_settings, update_app_settings};
use super::types::GraphicsBackendPreference;

#[cfg(test)]
use super::types::default_windows_graphics_backend;

pub fn load_windows_graphics_backend_preference() -> GraphicsBackendPreference {
    load_app_settings().windows_graphics_backend
}

pub fn save_windows_graphics_backend_preference(
    preference: GraphicsBackendPreference,
) -> Result<()> {
    update_app_settings(|settings| {
        settings.windows_graphics_backend = preference;
    })
}

pub fn windows_graphics_backend_preference_for_ui() -> Option<GraphicsBackendPreference> {
    if cfg!(target_os = "windows") {
        Some(load_windows_graphics_backend_preference())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{default_windows_graphics_backend, GraphicsBackendPreference};

    #[test]
    fn windows_default_backend_is_vulkan() {
        assert_eq!(
            default_windows_graphics_backend(),
            GraphicsBackendPreference::Vulkan
        );
    }

    #[test]
    fn graphics_backend_toggle_switches_between_variants() {
        assert_eq!(
            GraphicsBackendPreference::Vulkan.toggled(),
            GraphicsBackendPreference::Dx12
        );
        assert_eq!(
            GraphicsBackendPreference::Dx12.toggled(),
            GraphicsBackendPreference::Vulkan
        );
    }
}
