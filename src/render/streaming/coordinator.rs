use crate::render::context::state::RenderContext;

use super::contracts::{
    CanvasMediaSlotWorkPipeline, PreviewPass, PreviewPipeline, PreviewRoute, VideoPipeline,
    VideoRoute, VisibleRequest,
};
use super::{
    canvas_media_slots::CanvasMediaSlotPipeline, preview::PreviewImagePipeline,
    video::VideoRequestPipeline,
};

const PREVIEW_PIPELINE: PreviewImagePipeline = PreviewImagePipeline;
const CANVAS_MEDIA_SLOT_PIPELINE: CanvasMediaSlotPipeline = CanvasMediaSlotPipeline;
const VIDEO_PIPELINE: VideoRequestPipeline = VideoRequestPipeline;

pub fn handle_preview_coverage_request(ctx: &mut RenderContext, id: u64, item_idx: usize) {
    let _ = PREVIEW_PIPELINE.process(ctx, VisibleRequest { id, item_idx }, PreviewPass::Coverage);
}

pub fn handle_preview_quality_request(ctx: &mut RenderContext, id: u64, item_idx: usize) {
    let _ = PREVIEW_PIPELINE.process(ctx, VisibleRequest { id, item_idx }, PreviewPass::Quality);
}

pub fn handle_request(ctx: &mut RenderContext, id: u64, item_idx: usize) {
    let request = VisibleRequest { id, item_idx };
    let route = PREVIEW_PIPELINE.process(ctx, request, PreviewPass::Full);

    if let PreviewRoute::ToCanvasMediaSlots(input) = route {
        CANVAS_MEDIA_SLOT_PIPELINE.process(ctx, input);
    }

    let video_route = VIDEO_PIPELINE.process(ctx, request);
    if let VideoRoute::ToDecode(_input) = video_route {
        // Video decode queue is not wired yet.
    }
}
