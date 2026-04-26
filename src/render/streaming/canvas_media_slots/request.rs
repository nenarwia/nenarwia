use crate::render::context::state::RenderContext;
use crate::render::streaming::common::sampling::{is_undersampled, sample_mode};
use crate::render::streaming::contracts::{CanvasMediaSlotWorkInput, CanvasMediaSlotWorkPipeline};
use crate::render::streaming::gpu_sync::*;

use super::calculator::*;

mod lod_selection;
mod regions;
mod scheduling;

use lod_selection::{compute_desired_lod_state, LodSelectionInput};
use regions::{build_region_plan, RegionPlanInput};
use scheduling::{schedule_tile_requests, SchedulingInput};

const FEEDBACK_MAX_LATENCY_FRAMES: u64 = 3;

#[derive(Clone, Copy, Debug, Default)]
pub struct CanvasMediaSlotImagePipeline;

impl CanvasMediaSlotWorkPipeline for CanvasMediaSlotImagePipeline {
    fn process(&self, ctx: &mut RenderContext, input: CanvasMediaSlotWorkInput) {
        let CanvasMediaSlotWorkInput {
            id,
            item_idx,
            asset_key,
            orig_w,
            orig_h,
            obj_x,
            obj_y,
            obj_w,
            obj_h,
            obj_px_w,
            obj_px_h,
            max_px,
            desired_tier_px,
            thumb_undersampled,
        } = input;

        if item_idx >= ctx.scene.all_items_raw.len() {
            return;
        }
        if orig_w == 0 || orig_h == 0 {
            return;
        }

        let desired = compute_desired_lod_state(
            ctx,
            LodSelectionInput {
                id,
                item_idx,
                asset_key,
                orig_w,
                orig_h,
                obj_x,
                obj_y,
                obj_w,
                obj_h,
                obj_px_w,
                obj_px_h,
                full: true,
            },
        );
        let desired_lod = desired.desired_lod;
        let max_lod = desired.max_lod;
        let cap_lod = desired.cap_lod;
        let desired_w = desired.desired_w;
        let desired_h = desired.desired_h;
        let desired_vis = desired.desired_vis;
        let desired_tiles_x = desired.desired_tiles_x;
        let desired_tiles_y = desired.desired_tiles_y;
        let desired_tiles_x_f = desired.desired_tiles_x_f;
        let desired_tiles_y_f = desired.desired_tiles_y_f;
        let desired_complete = desired.desired_complete;
        let debt_boost = desired.debt_boost;

        let plan = build_region_plan(
            ctx,
            RegionPlanInput {
                full: true,
                item_idx,
                asset_key,
                orig_w,
                orig_h,
                obj_x,
                obj_y,
                obj_w,
                obj_h,
                obj_px_w,
                obj_px_h,
                max_px,
                desired_tier_px,
                desired_lod,
                max_lod,
                cap_lod,
                desired_w,
                desired_h,
                desired_tiles_x,
                desired_tiles_y,
                desired_tiles_x_f,
                desired_tiles_y_f,
                desired_vis: &desired_vis,
                desired_complete,
            },
        );
        let render_lod = plan.render_lod;
        let render_region = plan.render_region;
        let render_w = plan.render_w;
        let render_h = plan.render_h;
        let render_tiles_x_f = plan.render_tiles_x_f;
        let render_tiles_y_f = plan.render_tiles_y_f;
        let coarse_pass_region = plan.coarse_pass_region;
        let coarse_pass_tiles_x_f = plan.coarse_pass_tiles_x_f;
        let coarse_pass_tiles_y_f = plan.coarse_pass_tiles_y_f;
        let coarse_pass_lod = plan.coarse_pass_lod;
        let coarse_lod = plan.coarse_lod;
        let coarse_tiles_x = plan.coarse_tiles_x;
        let coarse_tiles_y = plan.coarse_tiles_y;
        let coarse_vis = plan.coarse_vis;
        let render_vis = plan.render_vis;

        let desired_undersample = is_undersampled(obj_px_w, obj_px_h, render_w, render_h);
        let desired_mode = sample_mode(desired_undersample);

        let coarse_undersample = if coarse_pass_region.is_some() && coarse_pass_lod != u8::MAX {
            let (coarse_w, coarse_h, _, _, _, _) = lod_info(orig_w, orig_h, coarse_pass_lod);
            is_undersampled(obj_px_w, obj_px_h, coarse_w, coarse_h)
        } else {
            false
        };
        let coarse_mode = sample_mode(coarse_undersample);

        let atlas_mode = sample_mode(thumb_undersampled);
        let sample_flags = [desired_mode, coarse_mode, atlas_mode, 0.0];

        update_instance_params_if_changed(
            ctx,
            InstanceParamUpdate {
                item_idx,
                desired: InstanceLayerParams {
                    region: render_region,
                    tiles_x: render_tiles_x_f,
                    tiles_y: render_tiles_y_f,
                },
                coarse: InstanceLayerParams {
                    region: coarse_pass_region,
                    tiles_x: coarse_pass_tiles_x_f,
                    tiles_y: coarse_pass_tiles_y_f,
                },
                sample_flags,
            },
        );

        if item_idx < ctx.scene.render_lod.len() {
            ctx.scene.render_lod[item_idx] = if render_region.is_some() {
                render_lod
            } else {
                u8::MAX
            };
        }
        if item_idx < ctx.scene.coarse_lod.len() {
            ctx.scene.coarse_lod[item_idx] = if coarse_pass_region.is_some() {
                coarse_pass_lod
            } else {
                u8::MAX
            };
        }

        schedule_tile_requests(
            ctx,
            SchedulingInput {
                id,
                item_idx,
                asset_key,
                render_lod,
                desired_lod,
                render_vis: &render_vis,
                desired_vis: &desired_vis,
                desired_complete,
                coarse_vis: &coarse_vis,
                coarse_lod,
                coarse_tiles_x,
                coarse_tiles_y,
                desired_tiles_x,
                desired_tiles_y,
                debt_boost,
                feedback_max_latency_frames: FEEDBACK_MAX_LATENCY_FRAMES,
            },
        );
    }
}
