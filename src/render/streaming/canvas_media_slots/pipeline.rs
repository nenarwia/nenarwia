use crate::render::context::state::RenderContext;
use crate::render::streaming::contracts::{CanvasMediaSlotWorkInput, CanvasMediaSlotWorkPipeline};

use super::CanvasMediaSlotImagePipeline;

const IMAGE_SLOT_PIPELINE_BRIDGE: CanvasMediaSlotImagePipeline = CanvasMediaSlotImagePipeline;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CanvasMediaSlotPipeline;

impl CanvasMediaSlotWorkPipeline for CanvasMediaSlotPipeline {
    fn process(&self, ctx: &mut RenderContext, input: CanvasMediaSlotWorkInput) {
        // Canvas media slots still reuse the shared image request path until
        // this domain owns cache-flow orchestration end-to-end.
        IMAGE_SLOT_PIPELINE_BRIDGE.process(ctx, input);
    }
}
