use super::constants::{TAB_CLOSE_SIDE_INSET_PX, TAB_CLOSE_SIZE_PX, TAB_CLOSE_TEXT_GAP_PX};

pub(super) fn inset_rect(rect: [u32; 4], inset_x: u32, inset_y: u32) -> [u32; 4] {
    let x = rect[0].saturating_add(inset_x);
    let y = rect[1].saturating_add(inset_y);
    let w = rect[2].saturating_sub(inset_x.saturating_mul(2));
    let h = rect[3].saturating_sub(inset_y.saturating_mul(2));
    [x, y, w, h]
}

pub(super) fn compute_tab_close_rect(rect: [u32; 4]) -> [u32; 4] {
    let x = rect[0]
        .saturating_add(rect[2])
        .saturating_sub(TAB_CLOSE_SIDE_INSET_PX)
        .saturating_sub(TAB_CLOSE_SIZE_PX);
    let y = rect[1].saturating_add((rect[3].saturating_sub(TAB_CLOSE_SIZE_PX)) / 2);
    [
        x,
        y,
        TAB_CLOSE_SIZE_PX.min(rect[2]),
        TAB_CLOSE_SIZE_PX.min(rect[3]),
    ]
}

pub(super) fn tab_title_rect(rect: [u32; 4], close_rect: Option<[u32; 4]>) -> [u32; 4] {
    match close_rect {
        Some(close_rect) => [
            rect[0],
            rect[1],
            close_rect[0]
                .saturating_sub(rect[0])
                .saturating_sub(TAB_CLOSE_TEXT_GAP_PX),
            rect[3],
        ],
        None => rect,
    }
}

pub(super) fn circle_rect(cx: i32, cy: i32, radius: i32, width: u32, height: u32) -> [u32; 4] {
    let x = (cx - radius).max(0) as u32;
    let y = (cy - radius).max(0) as u32;
    let w = (radius * 2).max(0) as u32;
    let h = (radius * 2).max(0) as u32;
    let max_w = width.saturating_sub(x);
    let max_h = height.saturating_sub(y);
    [x, y, w.min(max_w), h.min(max_h)]
}
