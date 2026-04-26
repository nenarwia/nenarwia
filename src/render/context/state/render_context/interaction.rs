use std::time::Instant;

use winit::dpi::PhysicalPosition;

#[derive(Clone, Copy, Debug)]
pub struct PendingCanvasClick {
    pub origin: PhysicalPosition<f64>,
    pub candidate_id: Option<u64>,
    pub dragged: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct EmptySlotClickStamp {
    pub slot_id: u64,
    pub at: Instant,
    pub pos: PhysicalPosition<f64>,
}

#[derive(Clone, Copy, Debug)]
pub struct MediaItemClickStamp {
    pub media_id: u64,
    pub at: Instant,
    pub pos: PhysicalPosition<f64>,
}
