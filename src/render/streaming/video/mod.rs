//! Streaming slot reserved for video pipeline.
//! Keep this module isolated from image preview/tile paths so video can be
//! added later as an independent component.

mod request;

pub use request::VideoRequestPipeline;
