use crate::render::context::state::RenderContext;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VisibleRequest {
    pub id: u64,
    pub item_idx: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreviewPass {
    Coverage,
    Quality,
    Full,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PreviewRoute {
    Consumed,
    ToCanvasMediaSlots(CanvasMediaSlotWorkInput),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasMediaSlotWorkInput {
    pub id: u64,
    pub item_idx: usize,
    pub asset_key: u64,
    pub orig_w: u32,
    pub orig_h: u32,
    pub obj_x: f64,
    pub obj_y: f64,
    pub obj_w: f32,
    pub obj_h: f32,
    pub obj_px_w: f32,
    pub obj_px_h: f32,
    pub max_px: f32,
    pub desired_tier_px: f32,
    pub thumb_undersampled: bool,
}

pub trait PreviewPipeline {
    fn process(
        &self,
        ctx: &mut RenderContext,
        request: VisibleRequest,
        pass: PreviewPass,
    ) -> PreviewRoute;
}

pub trait CanvasMediaSlotWorkPipeline {
    fn process(&self, ctx: &mut RenderContext, input: CanvasMediaSlotWorkInput);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VideoRoute {
    Consumed,
    ToDecode(VideoWorkInput),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VideoWorkInput {
    pub id: u64,
    pub item_idx: usize,
}

pub trait VideoPipeline {
    fn process(&self, ctx: &mut RenderContext, request: VisibleRequest) -> VideoRoute;
}
