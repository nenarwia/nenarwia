use crate::render::cache::directory::PtRegion;
use crate::render::context::state::RenderContext;

#[derive(Clone, Copy, Debug)]
pub struct InstanceLayerParams {
    pub region: Option<PtRegion>,
    pub tiles_x: f32,
    pub tiles_y: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct InstanceParamUpdate {
    pub item_idx: usize,
    pub desired: InstanceLayerParams,
    pub coarse: InstanceLayerParams,
    pub sample_flags: [f32; 4],
}

pub fn update_instance_params_if_changed(
    ctx: &mut RenderContext,
    input: InstanceParamUpdate,
) -> bool {
    let InstanceParamUpdate {
        item_idx,
        desired,
        coarse,
        sample_flags,
    } = input;

    if item_idx >= ctx.scene.all_items_raw.len() {
        return false;
    }

    // Keep mutable access scoped to this block to avoid borrow conflicts.
    let raw = &mut ctx.scene.all_items_raw[item_idx];
    let old_params = raw.params;
    let old_params2 = raw.params2;
    let old_flags = raw.sample_flags;

    write_layer_params(&mut raw.params, desired);
    write_layer_params(&mut raw.params2, coarse);

    raw.sample_flags = sample_flags;

    (raw.params != old_params) || (raw.params2 != old_params2) || (raw.sample_flags != old_flags)
}

pub fn update_instance_params_desired_only(
    ctx: &mut RenderContext,
    item_idx: usize,
    desired: InstanceLayerParams,
    sample_flags: [f32; 4],
) -> bool {
    if item_idx >= ctx.scene.all_items_raw.len() {
        return false;
    }

    let raw = &mut ctx.scene.all_items_raw[item_idx];
    let old_params = raw.params;
    let old_flags = raw.sample_flags;

    write_layer_params(&mut raw.params, desired);

    raw.sample_flags = sample_flags;

    (raw.params != old_params) || (raw.sample_flags != old_flags)
}

fn write_layer_params(dst: &mut [f32; 4], layer: InstanceLayerParams) {
    if let Some(region) = layer.region {
        dst[0] = region.x as f32;
        dst[1] = region.y as f32;
        dst[2] = layer.tiles_x;
        dst[3] = layer.tiles_y;
    } else {
        dst[0] = -1.0;
        dst[1] = -1.0;
        dst[2] = 0.0;
        dst[3] = 0.0;
    }
}
