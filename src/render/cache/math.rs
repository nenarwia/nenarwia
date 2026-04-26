use super::constants::TILE_SIZE;

pub struct VisibleTiles {
    pub min_tx: u32,
    pub max_tx: u32,
    pub min_ty: u32,
    pub max_ty: u32,
}

pub fn calculate_visible_tiles_f64(
    viewport: (f64, f64, f64, f64),
    item_x: f64,
    item_y: f64,
    item_w: f32,
    item_h: f32,
    image_w: u32,
    image_h: u32,
) -> Option<VisibleTiles> {
    let (view_min_x, view_max_x, view_min_y, view_max_y) = viewport;
    let item_w = item_w.max(0.0) as f64;
    let item_h = item_h.max(0.0) as f64;
    if item_w <= 0.0 || item_h <= 0.0 {
        return None;
    }

    let obj_left = item_x - item_w * 0.5;
    let obj_right = item_x + item_w * 0.5;
    let obj_top = item_y + item_h * 0.5;
    let obj_bottom = item_y - item_h * 0.5;
    let visible_left = obj_left.max(view_min_x);
    let visible_right = obj_right.min(view_max_x);
    let visible_bottom = obj_bottom.max(view_min_y);
    let visible_top = obj_top.min(view_max_y);

    if visible_left >= visible_right || visible_bottom >= visible_top {
        return None;
    }

    let u_min = ((visible_left - obj_left) / item_w).clamp(0.0, 1.0);
    let u_max = ((visible_right - obj_left) / item_w).clamp(0.0, 1.0);
    let v_min = ((obj_top - visible_top) / item_h).clamp(0.0, 1.0);
    let v_max = ((obj_top - visible_bottom) / item_h).clamp(0.0, 1.0);
    let tiles_x = (image_w as f64 / TILE_SIZE as f64).ceil() as u32;
    let tiles_y = (image_h as f64 / TILE_SIZE as f64).ceil() as u32;

    let min_tx = (u_min * tiles_x as f64).floor() as u32;
    let max_tx = (u_max * tiles_x as f64).ceil() as u32;
    let min_ty = (v_min * tiles_y as f64).floor() as u32;
    let max_ty = (v_max * tiles_y as f64).ceil() as u32;
    Some(VisibleTiles {
        min_tx: min_tx.min(tiles_x.saturating_sub(1)),
        max_tx: max_tx.min(tiles_x),
        min_ty: min_ty.min(tiles_y.saturating_sub(1)),
        max_ty: max_ty.min(tiles_y),
    })
}
