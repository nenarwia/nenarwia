pub mod calculator;
pub mod scheduler;
pub mod visibility;

mod eviction;
mod pipeline;
mod request;

pub(crate) use pipeline::CanvasMediaSlotPipeline;
pub(crate) use request::CanvasMediaSlotImagePipeline;
pub(crate) use scheduler::{drain_canvas_media_slot_queue, enqueue_canvas_media_slot_request};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CanvasMediaSlotQueueItem {
    pub id: u64,
    pub asset_key: u64,
    pub item_idx: usize,
    pub lod: u8,
    pub x: u32,
    pub y: u32,
    pub prio: i32,
    pub is_prefetch: bool,
    pub epoch: u64,
    pub queued_frame: u64,
}

impl Ord for CanvasMediaSlotQueueItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.prio
            .cmp(&other.prio)
            .then_with(|| other.queued_frame.cmp(&self.queued_frame))
            .then_with(|| self.asset_key.cmp(&other.asset_key))
            .then_with(|| self.id.cmp(&other.id))
            .then_with(|| self.lod.cmp(&other.lod))
            .then_with(|| self.x.cmp(&other.x))
            .then_with(|| self.y.cmp(&other.y))
            .then_with(|| self.item_idx.cmp(&other.item_idx))
            .then_with(|| self.is_prefetch.cmp(&other.is_prefetch))
            .then_with(|| self.epoch.cmp(&other.epoch))
    }
}

impl PartialOrd for CanvasMediaSlotQueueItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
