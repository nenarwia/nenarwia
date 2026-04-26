mod ordering;
mod pending;
mod planner;
mod preview_checks;
mod residency;
mod slot_gate;

use std::time::Duration;

use crate::render::context::state::RenderContext;
use crate::render::streaming as render_streaming;

pub(crate) use slot_gate::force_disable_slot_interaction_gate;

const PREVIEW_RING_COUNT: usize = 8;
const PREVIEW_RING_WEIGHTS_MOVING: [usize; PREVIEW_RING_COUNT] = [8, 6, 4, 3, 2, 1, 1, 1];
const PREVIEW_RING_WEIGHTS_IDLE: [usize; PREVIEW_RING_COUNT] = [4, 4, 3, 3, 2, 2, 1, 1];
const PREVIEW_COVERAGE_LEAD_FRAMES_MOVING: f32 = 10.0;
const PREVIEW_COVERAGE_LEAD_FRAMES_IDLE: f32 = 28.0;
const PREVIEW_RING_LOCK_MOVING_BAND: usize = 0;
const PREVIEW_RING_LOCK_IDLE_BAND: usize = 1;
const PREVIEW_COVERAGE_MOVING_QUEUE_FLOOR_FRAMES: usize = 4;
const PREVIEW_COVERAGE_MOVING_QUEUE_CAP_FRAMES: usize = 6;

pub(crate) fn prepare_frame(ctx: &mut RenderContext, frame_dt: Duration) {
    let moving_recently = ctx.viewport_runtime().moving_recently;
    ctx.streaming_runtime
        .prepare_frame_budgets(&ctx.streaming, moving_recently, frame_dt);
}

pub(crate) fn process_committed_view(ctx: &mut RenderContext) {
    planner::process_committed_view(ctx);
}

pub(crate) fn drain_queued_requests(ctx: &mut RenderContext) {
    render_streaming::drain_canvas_media_slot_queue(ctx);
}
