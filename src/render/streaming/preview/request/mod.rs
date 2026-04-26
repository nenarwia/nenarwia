mod context;
mod scoring;

use crate::render::context::state::RenderContext;
use crate::render::streaming::contracts::{
    CanvasMediaSlotWorkInput, PreviewPass, PreviewPipeline, PreviewRoute, VisibleRequest,
};

use self::context::{build_preview_request_context, PreviewRequestContext};
use super::{handle_atlas_path, handle_missing_dimensions, EarlyRequestInput};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PreviewRequestMode {
    Coverage,
    Quality,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PreviewImagePipeline;

impl PreviewPipeline for PreviewImagePipeline {
    fn process(
        &self,
        ctx: &mut RenderContext,
        request: VisibleRequest,
        pass: PreviewPass,
    ) -> PreviewRoute {
        match pass {
            PreviewPass::Coverage => {
                handle_preview_request(ctx, request, PreviewRequestMode::Coverage);
                PreviewRoute::Consumed
            }
            PreviewPass::Quality => {
                handle_preview_request(ctx, request, PreviewRequestMode::Quality);
                PreviewRoute::Consumed
            }
            PreviewPass::Full => handle_full_request(ctx, request),
        }
    }
}

fn handle_full_request(ctx: &mut RenderContext, request: VisibleRequest) -> PreviewRoute {
    let Some(req) = build_preview_request_context(ctx, request.item_idx) else {
        return PreviewRoute::Consumed;
    };

    if req.thumb_undersampled {
        ctx.quality_stats
            .record_tier_downgrade(req.thumb_undersample);
    }

    let early = build_early_request_input(request, req, true, false, false);

    if handle_missing_dimensions(ctx, early, req.orig_w, req.orig_h) {
        return PreviewRoute::Consumed;
    }

    if handle_atlas_path(ctx, early) {
        return PreviewRoute::Consumed;
    }

    if req.orig_w == 0 || req.orig_h == 0 {
        return PreviewRoute::Consumed;
    }

    PreviewRoute::ToCanvasMediaSlots(CanvasMediaSlotWorkInput {
        id: request.id,
        item_idx: request.item_idx,
        asset_key: req.asset_key,
        orig_w: req.orig_w,
        orig_h: req.orig_h,
        obj_x: req.obj_x,
        obj_y: req.obj_y,
        obj_w: req.obj_w,
        obj_h: req.obj_h,
        obj_px_w: req.obj_px_w,
        obj_px_h: req.obj_px_h,
        max_px: req.max_px,
        desired_tier_px: req.desired_tier_px,
        thumb_undersampled: req.thumb_undersampled,
    })
}

fn handle_preview_request(
    ctx: &mut RenderContext,
    request: VisibleRequest,
    mode: PreviewRequestMode,
) {
    let Some(req) = build_preview_request_context(ctx, request.item_idx) else {
        return;
    };

    let early = build_early_request_input(
        request,
        req,
        false,
        true,
        mode == PreviewRequestMode::Quality,
    );

    if handle_missing_dimensions(ctx, early, req.orig_w, req.orig_h) {
        return;
    }

    let _ = handle_atlas_path(ctx, early);
}

#[inline]
fn build_early_request_input(
    request: VisibleRequest,
    req: PreviewRequestContext,
    full: bool,
    allow_requests: bool,
    allow_quality_requests: bool,
) -> EarlyRequestInput {
    EarlyRequestInput {
        id: request.id,
        item_idx: request.item_idx,
        full,
        allow_requests,
        allow_quality_requests,
        thumb_prio_coverage: req.thumb_prio_coverage,
        thumb_prio_quality: req.thumb_prio_quality,
        coverage_tier: req.coverage_tier,
        desired_tier: req.desired_tier,
        desired_tier_px: req.desired_tier_px,
        max_px: req.max_px,
        thumb_undersampled: req.thumb_undersampled,
    }
}
