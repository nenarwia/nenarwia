mod graphics;
mod persistence;
mod types;

pub use graphics::{
    load_windows_graphics_backend_preference, save_windows_graphics_backend_preference,
    windows_graphics_backend_preference_for_ui,
};
#[allow(unused_imports)]
pub use persistence::load_app_settings;
#[allow(unused_imports)]
pub use types::AppSettings;
pub use types::GraphicsBackendPreference;
