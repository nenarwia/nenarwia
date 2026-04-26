use crate::render::context::state::RenderContext;

pub const VIEWPORT_DIST_WEIGHT_VISIBLE: i32 = 3;
pub const VIEWPORT_DIST_WEIGHT_PREFETCH: i32 = 50;
pub const VIEWPORT_DIST_VISIBLE_MAX_PENALTY: i32 = 12_000;
pub const VIEWPORT_DIST_PREFETCH_MAX_PENALTY: i32 = 250_000;
pub const QUALITY_DEBT_BUCKET_SCALE: f32 = 10.0;
pub const QUALITY_DEBT_BUCKET_WEIGHT: i32 = 10_000_000;

#[derive(Clone, Copy, Debug)]
pub struct TileDistanceInput {
    pub obj_x: f64,
    pub obj_y: f64,
    pub obj_w: f32,
    pub obj_h: f32,
    pub tiles_x: u32,
    pub tiles_y: u32,
    pub tx: u32,
    pub ty: u32,
}

pub fn world_size_to_pixels(ctx: &RenderContext, world_w: f32, world_h: f32) -> (f32, f32) {
    ctx.view_metrics().world_size_to_pixels(world_w, world_h)
}

pub fn media_world_geometry(ctx: &RenderContext, item_idx: usize) -> Option<(f64, f64, f32, f32)> {
    ctx.scene.item_fitted_world_geometry(item_idx)
}

pub fn media_world_size_to_pixels(ctx: &RenderContext, item_idx: usize) -> Option<(f32, f32)> {
    let (_, _, world_w, world_h) = ctx.scene.item_fitted_world_geometry(item_idx)?;
    Some(world_size_to_pixels(ctx, world_w, world_h))
}

pub fn tile_center_distance_px(ctx: &RenderContext, input: TileDistanceInput) -> f32 {
    let TileDistanceInput {
        obj_x,
        obj_y,
        obj_w,
        obj_h,
        tiles_x,
        tiles_y,
        tx,
        ty,
    } = input;

    if tiles_x == 0 || tiles_y == 0 {
        return 0.0;
    }

    let obj_w = obj_w.abs();
    let obj_h = obj_h.abs();
    if obj_w <= 0.0 || obj_h <= 0.0 {
        return 0.0;
    }

    let tile_w = obj_w / tiles_x as f32;
    let tile_h = obj_h / tiles_y as f32;
    let left = obj_x - obj_w as f64 * 0.5;
    let top = obj_y + obj_h as f64 * 0.5;
    let center_x = left + (tx as f64 + 0.5) * tile_w as f64;
    let center_y = top - (ty as f64 + 0.5) * tile_h as f64;

    ctx.view_metrics()
        .point_distance_to_center_l1_px(center_x, center_y)
}

pub fn tile_tiebreak_bias(tx: u32, ty: u32) -> i32 {
    let mut h = tx.wrapping_mul(0x9E37_79B1) ^ ty.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 16;
    (h & 0x3FF) as i32
}

pub fn viewport_dist_penalty(dist_px: i32, weight: i32, max_penalty: i32) -> i32 {
    if weight <= 0 || dist_px <= 0 {
        return 0;
    }
    dist_px.saturating_mul(weight).min(max_penalty.max(0))
}

pub fn choose_lod_hysteresis(
    orig_w: u32,
    orig_h: u32,
    obj_px_w: f32,
    obj_px_h: f32,
    prev_lod: u8,
) -> (u8, u8) {
    let obj_px_w = obj_px_w.max(1.0);
    let obj_px_h = obj_px_h.max(1.0);

    let ratio_w = orig_w as f32 / obj_px_w;
    let ratio_h = orig_h as f32 / obj_px_h;
    let ratio = ratio_w.min(ratio_h);

    let mut lod = if ratio <= 1.0 {
        0u8
    } else {
        ratio.log2().floor().max(0.0) as u8
    };

    let max_dim = orig_w.max(orig_h) as f32;
    let max_lod = (max_dim / 256.0).log2().ceil().max(0.0) as u8;

    if lod > max_lod {
        lod = max_lod;
    }

    // Non-zero hysteresis reduces LOD thrash near zoom boundaries.
    const H_UP: f32 = 0.12;
    const H_DOWN: f32 = 0.12;

    let mut out = lod;
    if out > prev_lod {
        let pow = (1u32 << ((prev_lod as u32 + 1).min(31))) as f32;
        if ratio < pow * (1.0 + H_UP) {
            out = prev_lod;
        }
    } else if out < prev_lod {
        let pow = (1u32 << ((prev_lod as u32).min(31))) as f32;
        if ratio > pow * (1.0 - H_DOWN) {
            out = prev_lod;
        }
    }

    (out, max_lod)
}

pub fn cap_lod_for_screen(orig_w: u32, orig_h: u32, obj_px_w: f32, obj_px_h: f32) -> Option<u8> {
    let max_px = obj_px_w.max(obj_px_h);
    let cap_px = if max_px <= 64.0 {
        64u32
    } else if max_px <= 128.0 {
        128u32
    } else if max_px <= 256.0 {
        256u32
    } else {
        return None;
    };

    let mut size = orig_w.max(orig_h).max(1);
    let mut lod = 0u8;
    while size > cap_px {
        size = div_ceil(size, 2).max(1);
        lod = lod.saturating_add(1);
        if lod == u8::MAX {
            break;
        }
    }
    Some(lod)
}

pub fn required_lod_for_screen(orig_w: u32, orig_h: u32, obj_px_w: f32, obj_px_h: f32) -> u8 {
    let obj_px_w = obj_px_w.max(1.0);
    let obj_px_h = obj_px_h.max(1.0);

    let ratio_w = orig_w as f32 / obj_px_w;
    let ratio_h = orig_h as f32 / obj_px_h;
    let ratio = ratio_w.min(ratio_h);

    if ratio <= 1.0 {
        0u8
    } else {
        ratio.log2().floor().max(0.0) as u8
    }
}

pub fn lod_info(orig_w: u32, orig_h: u32, lod: u8) -> (u32, u32, u32, u32, f32, f32) {
    let shift = (lod as u32).min(31);
    let scale = 1u32 << shift;
    let lod_w = div_ceil(orig_w, scale).max(1);
    let lod_h = div_ceil(orig_h, scale).max(1);
    let tiles_x = div_ceil(lod_w, 256);
    let tiles_y = div_ceil(lod_h, 256);
    let tiles_x_f = lod_w as f32 / 256.0;
    let tiles_y_f = lod_h as f32 / 256.0;
    (lod_w, lod_h, tiles_x, tiles_y, tiles_x_f, tiles_y_f)
}

pub fn div_ceil(a: u32, b: u32) -> u32 {
    if b == 0 {
        return 0;
    }
    a.div_ceil(b)
}
