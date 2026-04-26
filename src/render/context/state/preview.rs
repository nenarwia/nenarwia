use crate::render::atlas::{ThumbClass, ThumbTier};
pub use crate::spatial::navigation::PreviewMotionTier;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PendingThumbRequest {
    pub epoch: u64,
    pub class: ThumbClass,
    pub asset_key: u64,
    pub tier: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreviewTierState {
    pub target: ThumbTier,
    pub display: Option<ThumbTier>,
    pub pending: Option<ThumbTier>,
}
