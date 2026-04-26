use crate::render::ui::sidebar::nav::{visible_nav_items, MAX_NAV_ITEMS};
use crate::render::ui::{
    DEBUG_SLOT_TOGGLE_ENABLED, SIDEBAR_ITEM_GAP_PX, SIDEBAR_ITEM_HEIGHT_PX, SIDEBAR_PADDING_TOP_PX,
    SIDEBAR_PADDING_X_PX, SIDEBAR_WIDTH_PX,
};

pub(in crate::render::ui::sidebar::style::panel) struct SidebarPanelLayout {
    pub nav_item_rects: [[u32; 4]; MAX_NAV_ITEMS],
    pub debug_slot_backdrop_rect: [u32; 4],
    pub fps_toggle_rect: [u32; 4],
    pub backend_toggle_rect: [u32; 4],
    pub wallpaper_rect: [u32; 4],
    pub recent_section_top: u32,
    pub recent_wallpaper_rects: Vec<[u32; 4]>,
}

pub(in crate::render::ui::sidebar::style::panel) fn build_panel_layout(
    height: u32,
    has_backend_toggle: bool,
    recent_wallpaper_count: usize,
) -> SidebarPanelLayout {
    let width = SIDEBAR_WIDTH_PX;
    let item_width = width.saturating_sub(SIDEBAR_PADDING_X_PX * 2);
    let item_start_y = SIDEBAR_PADDING_TOP_PX;
    let mut nav_item_rects = [[0; 4]; MAX_NAV_ITEMS];
    for idx in 0..visible_nav_items().len() {
        nav_item_rects[idx] = [
            SIDEBAR_PADDING_X_PX,
            item_start_y + (SIDEBAR_ITEM_HEIGHT_PX + SIDEBAR_ITEM_GAP_PX) * idx as u32,
            item_width,
            SIDEBAR_ITEM_HEIGHT_PX,
        ];
    }
    let footer_bottom_pad = 16;
    let footer_gap = 8;
    let wallpaper_rect = [
        SIDEBAR_PADDING_X_PX,
        height
            .saturating_sub(SIDEBAR_ITEM_HEIGHT_PX)
            .saturating_sub(footer_bottom_pad),
        item_width,
        SIDEBAR_ITEM_HEIGHT_PX,
    ];
    let mut footer_cursor = wallpaper_rect[1].saturating_sub(SIDEBAR_ITEM_HEIGHT_PX + footer_gap);
    let backend_toggle_rect = if has_backend_toggle {
        let rect = [
            SIDEBAR_PADDING_X_PX,
            footer_cursor,
            item_width,
            SIDEBAR_ITEM_HEIGHT_PX,
        ];
        footer_cursor = rect[1].saturating_sub(SIDEBAR_ITEM_HEIGHT_PX + footer_gap);
        rect
    } else {
        [SIDEBAR_PADDING_X_PX, footer_cursor, item_width, 0]
    };
    let fps_toggle_rect = [
        SIDEBAR_PADDING_X_PX,
        footer_cursor,
        item_width,
        SIDEBAR_ITEM_HEIGHT_PX,
    ];
    let debug_slot_backdrop_rect = if DEBUG_SLOT_TOGGLE_ENABLED {
        [
            SIDEBAR_PADDING_X_PX,
            fps_toggle_rect[1].saturating_sub(SIDEBAR_ITEM_HEIGHT_PX + footer_gap),
            item_width,
            SIDEBAR_ITEM_HEIGHT_PX,
        ]
    } else {
        [SIDEBAR_PADDING_X_PX, fps_toggle_rect[1], item_width, 0]
    };
    let last_nav_rect = nav_item_rects[visible_nav_items().len().saturating_sub(1)];
    let recent_section_top = last_nav_rect[1]
        .saturating_add(last_nav_rect[3])
        .saturating_add(22);
    let footer_top = if DEBUG_SLOT_TOGGLE_ENABLED {
        debug_slot_backdrop_rect[1]
    } else {
        fps_toggle_rect[1]
    };
    let recent_section_bottom = footer_top.saturating_sub(20);
    let recent_wallpaper_rects = build_recent_wallpaper_rects(
        item_width,
        recent_section_top,
        recent_section_bottom,
        recent_wallpaper_count,
    );

    SidebarPanelLayout {
        nav_item_rects,
        debug_slot_backdrop_rect,
        fps_toggle_rect,
        backend_toggle_rect,
        wallpaper_rect,
        recent_section_top,
        recent_wallpaper_rects,
    }
}

fn build_recent_wallpaper_rects(
    item_width: u32,
    top: u32,
    bottom: u32,
    item_count: usize,
) -> Vec<[u32; 4]> {
    if item_count == 0 || bottom <= top {
        return Vec::new();
    }

    let visible = item_count.min(10);
    let cols = 2u32;
    let rows = (visible as u32).div_ceil(cols);
    let col_gap = 8u32;
    let row_gap = 8u32;
    let tile_width = item_width
        .saturating_sub(col_gap)
        .checked_div(cols)
        .unwrap_or(item_width)
        .max(1);
    let available_height = bottom.saturating_sub(top).saturating_sub(18);
    if available_height == 0 {
        return Vec::new();
    }
    let total_row_gap = row_gap.saturating_mul(rows.saturating_sub(1));
    let tile_height = available_height
        .saturating_sub(total_row_gap)
        .checked_div(rows.max(1))
        .unwrap_or(available_height)
        .clamp(1, 52);

    let mut rects = Vec::with_capacity(visible);
    let grid_top = top.saturating_add(18);
    for idx in 0..visible {
        let row = (idx as u32) / cols;
        let col = (idx as u32) % cols;
        rects.push([
            SIDEBAR_PADDING_X_PX + col * (tile_width + col_gap),
            grid_top + row * (tile_height + row_gap),
            tile_width,
            tile_height,
        ]);
    }
    rects
}

#[cfg(test)]
mod tests {
    use super::build_recent_wallpaper_rects;

    #[test]
    fn recent_wallpaper_rects_are_empty_without_items_or_space() {
        assert!(build_recent_wallpaper_rects(200, 100, 200, 0).is_empty());
        assert!(build_recent_wallpaper_rects(200, 200, 200, 4).is_empty());
        assert!(build_recent_wallpaper_rects(200, 220, 200, 4).is_empty());
    }

    #[test]
    fn recent_wallpaper_rects_cap_items_and_use_two_column_grid() {
        let rects = build_recent_wallpaper_rects(200, 100, 360, 14);

        assert_eq!(rects.len(), 10);
        assert_eq!(rects[0][1], rects[1][1]);
        assert!(rects[1][0] > rects[0][0]);
        assert!(rects[2][1] > rects[0][1]);
        assert_eq!(rects[2][0], rects[0][0]);
    }
}
