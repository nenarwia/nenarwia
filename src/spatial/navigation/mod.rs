mod controller;
mod input;
mod runtime;
mod viewport;

pub(crate) const SPRING_STIFFNESS: f64 = 6.5;
pub(crate) const SPRING_ANIMATION_TIME_SECS: f64 = 1.2;
pub(crate) const ZOOM_PER_SCROLL: f64 = 1.20;
pub(crate) const MIN_ZOOM_IMAGE_RATIO: f64 = 0.9;
pub(crate) const VISIBILITY_RATIO: f64 = 0.5;
pub(crate) const PIXELS_PER_WHEEL_LINE: f64 = 40.0;
pub(crate) const ZOOM_MIN: f64 = 1.0e-6;
pub(crate) const ZOOM_MAX: f64 = 8.0;

pub use controller::NavigationController;
pub use input::{apply_window_event_with_cursor, ViewportIntent};
pub use runtime::{PreviewMotionTier, ViewRuntime, ViewRuntimeConfig};
pub use viewport::ViewportState;
