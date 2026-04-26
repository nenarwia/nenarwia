pub mod allocator;
pub mod gpu;
pub mod manager;

pub use manager::TextureAtlas;
pub mod multi;

pub use multi::{MultiTierAtlas, ThumbClass, ThumbTier, ThumbnailUploadInput};
