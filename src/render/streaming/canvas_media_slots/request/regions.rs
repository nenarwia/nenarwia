use crate::render::cache::directory::PtRegion;
use crate::render::cache::math;
use crate::render::context::state::RenderContext;

use super::super::calculator::lod_info;
use super::super::eviction::process_evictions;

pub(super) struct RegionPlan {
    pub render_lod: u8,
    pub render_region: Option<PtRegion>,
    pub render_w: u32,
    pub render_h: u32,
    pub render_tiles_x_f: f32,
    pub render_tiles_y_f: f32,
    pub coarse_pass_region: Option<PtRegion>,
    pub coarse_pass_tiles_x_f: f32,
    pub coarse_pass_tiles_y_f: f32,
    pub coarse_pass_lod: u8,
    pub coarse_lod: u8,
    pub coarse_tiles_x: u32,
    pub coarse_tiles_y: u32,
    pub coarse_vis: Option<math::VisibleTiles>,
    pub render_vis: Option<math::VisibleTiles>,
}

pub(super) struct RegionPlanInput<'a> {
    pub full: bool,
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
    pub desired_lod: u8,
    pub max_lod: u8,
    pub cap_lod: Option<u8>,
    pub desired_w: u32,
    pub desired_h: u32,
    pub desired_tiles_x: u32,
    pub desired_tiles_y: u32,
    pub desired_tiles_x_f: f32,
    pub desired_tiles_y_f: f32,
    pub desired_vis: &'a Option<math::VisibleTiles>,
    pub desired_complete: bool,
}

pub(super) fn build_region_plan(ctx: &mut RenderContext, input: RegionPlanInput<'_>) -> RegionPlan {
    let full = input.full;
    let item_idx = input.item_idx;
    let desired_lod = input.desired_lod;
    let desired_complete = input.desired_complete;
    let view = ctx.view().viewport_rect();

    let desired_alloc = ctx.page_directory.ensure_region(
        &ctx.gpu.queue,
        input.asset_key,
        desired_lod,
        input.desired_tiles_x,
        input.desired_tiles_y,
    );

    if !desired_alloc.evicted.is_empty() {
        process_evictions(ctx, &desired_alloc.evicted);
    }
    let mut desired_region = ctx.page_directory.get_region(input.asset_key, desired_lod);

    if full
        && desired_complete
        && desired_region.is_some()
        && item_idx < ctx.scene.display_lod.len()
    {
        ctx.scene.display_lod[item_idx] = desired_lod;
    }

    let mut display_lod = ctx
        .scene
        .display_lod
        .get(item_idx)
        .copied()
        .unwrap_or(u8::MAX);
    if let Some(cap) = input.cap_lod {
        if display_lod < cap {
            display_lod = cap;
        }
    }

    let render_lod = if desired_complete {
        desired_lod
    } else if display_lod != u8::MAX {
        display_lod
    } else {
        desired_lod
    };

    let (render_w, render_h, render_tiles_x, render_tiles_y, render_tiles_x_f, render_tiles_y_f) =
        if render_lod == desired_lod {
            (
                input.desired_w,
                input.desired_h,
                input.desired_tiles_x,
                input.desired_tiles_y,
                input.desired_tiles_x_f,
                input.desired_tiles_y_f,
            )
        } else {
            lod_info(input.orig_w, input.orig_h, render_lod)
        };

    if render_lod != desired_lod {
        let alloc = ctx.page_directory.ensure_region(
            &ctx.gpu.queue,
            input.asset_key,
            render_lod,
            render_tiles_x,
            render_tiles_y,
        );

        if !alloc.evicted.is_empty() {
            process_evictions(ctx, &alloc.evicted);
        }
        desired_region = ctx.page_directory.get_region(input.asset_key, desired_lod);
    }
    let mut render_region = ctx.page_directory.get_region(input.asset_key, render_lod);

    let mut coarse_pass_region = None;
    let mut coarse_pass_tiles_x_f = 0.0;
    let mut coarse_pass_tiles_y_f = 0.0;
    let mut coarse_pass_lod = u8::MAX;
    let mut coarse_lod = u8::MAX;
    let mut coarse_tiles_x = 0u32;
    let mut coarse_tiles_y = 0u32;
    let mut coarse_vis: Option<math::VisibleTiles> = None;
    let mut render_vis: Option<math::VisibleTiles> = None;

    if full {
        coarse_lod = desired_lod.saturating_add(1).min(input.max_lod);
        let (coarse_w, coarse_h, c_tiles_x, c_tiles_y, coarse_tiles_x_local, coarse_tiles_y_local) =
            lod_info(input.orig_w, input.orig_h, coarse_lod);
        coarse_tiles_x = c_tiles_x;
        coarse_tiles_y = c_tiles_y;
        coarse_vis = math::calculate_visible_tiles_f64(
            view,
            input.obj_x,
            input.obj_y,
            input.obj_w,
            input.obj_h,
            coarse_w,
            coarse_h,
        );

        if render_lod != desired_lod {
            coarse_pass_region = desired_region;
            coarse_pass_tiles_x_f = input.desired_tiles_x_f;
            coarse_pass_tiles_y_f = input.desired_tiles_y_f;
            if coarse_pass_region.is_some() {
                coarse_pass_lod = desired_lod;
            }
        } else {
            let fallback_lod = if desired_complete {
                Some(desired_lod)
            } else if display_lod != u8::MAX && display_lod != desired_lod {
                Some(display_lod)
            } else if coarse_lod != desired_lod {
                Some(coarse_lod)
            } else {
                Some(desired_lod)
            };

            if let Some(flod) = fallback_lod {
                if flod == desired_lod {
                    coarse_pass_region = desired_region;
                    coarse_pass_tiles_x_f = input.desired_tiles_x_f;
                    coarse_pass_tiles_y_f = input.desired_tiles_y_f;
                    if coarse_pass_region.is_some() {
                        coarse_pass_lod = flod;
                    }
                } else if flod == coarse_lod {
                    let alloc = ctx.page_directory.ensure_region(
                        &ctx.gpu.queue,
                        input.asset_key,
                        coarse_lod,
                        coarse_tiles_x,
                        coarse_tiles_y,
                    );

                    if !alloc.evicted.is_empty() {
                        process_evictions(ctx, &alloc.evicted);
                    }
                    render_region = ctx.page_directory.get_region(input.asset_key, render_lod);

                    coarse_pass_region = alloc.region;
                    coarse_pass_tiles_x_f = coarse_tiles_x_local;
                    coarse_pass_tiles_y_f = coarse_tiles_y_local;
                    if coarse_pass_region.is_some() {
                        coarse_pass_lod = flod;
                    }
                } else {
                    let (_w, _h, tiles_x, tiles_y, tiles_x_f, tiles_y_f) =
                        lod_info(input.orig_w, input.orig_h, flod);
                    let alloc = ctx.page_directory.ensure_region(
                        &ctx.gpu.queue,
                        input.asset_key,
                        flod,
                        tiles_x,
                        tiles_y,
                    );

                    if !alloc.evicted.is_empty() {
                        process_evictions(ctx, &alloc.evicted);
                    }
                    render_region = ctx.page_directory.get_region(input.asset_key, render_lod);

                    coarse_pass_region = alloc.region;
                    coarse_pass_tiles_x_f = tiles_x_f;
                    coarse_pass_tiles_y_f = tiles_y_f;
                    if coarse_pass_region.is_some() {
                        coarse_pass_lod = flod;
                    }
                }
            }
        }

        render_vis = if render_lod == desired_lod {
            input.desired_vis.as_ref().map(|v| math::VisibleTiles {
                min_tx: v.min_tx,
                max_tx: v.max_tx,
                min_ty: v.min_ty,
                max_ty: v.max_ty,
            })
        } else {
            math::calculate_visible_tiles_f64(
                view,
                input.obj_x,
                input.obj_y,
                input.obj_w,
                input.obj_h,
                render_w,
                render_h,
            )
        };
    }

    if full && coarse_pass_lod != u8::MAX {
        coarse_pass_region = ctx
            .page_directory
            .get_region(input.asset_key, coarse_pass_lod);
        if coarse_pass_region.is_none() {
            coarse_pass_lod = u8::MAX;
            coarse_pass_tiles_x_f = 0.0;
            coarse_pass_tiles_y_f = 0.0;
        }
    }

    if full {
        let undersample = if render_region.is_some() {
            let rw = render_w.max(1) as f32;
            let rh = render_h.max(1) as f32;
            (input.obj_px_w / rw).max(input.obj_px_h / rh).max(1.0)
        } else if input.desired_tier_px > 0.0 {
            (input.max_px / input.desired_tier_px).max(1.0)
        } else {
            1.0
        };
        ctx.quality_stats.record_visible_undersample(undersample);
    }

    RegionPlan {
        render_lod,
        render_region,
        render_w,
        render_h,
        render_tiles_x_f,
        render_tiles_y_f,
        coarse_pass_region,
        coarse_pass_tiles_x_f,
        coarse_pass_tiles_y_f,
        coarse_pass_lod,
        coarse_lod,
        coarse_tiles_x,
        coarse_tiles_y,
        coarse_vis,
        render_vis,
    }
}
