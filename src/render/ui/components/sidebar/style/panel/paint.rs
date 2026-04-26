use crate::render::ui::sidebar::nav::visible_nav_items;
use crate::render::ui::sidebar::state::SidebarSavedWallpaperItem;
use crate::render::ui::{CHROME_HEIGHT_PX, DEBUG_SLOT_TOGGLE_ENABLED};

use super::layout::SidebarPanelLayout;
use super::thumbs::{blit_cover_bilinear, inset_rect};
use crate::render::ui::sidebar::style::constants::{
    SIDEBAR_BG_COLOR, SIDEBAR_HOVER_COLOR, SIDEBAR_ITEM_RADIUS_PX,
};
use crate::render::ui::sidebar::style::primitives::{
    fill_rect_region, fill_rounded_rect_region_aa,
};

pub(in crate::render::ui::sidebar::style::panel) fn paint_panel(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    layout: &SidebarPanelLayout,
    hovered_nav_item: Option<usize>,
    hovered_debug_slot_backdrop: bool,
    hovered_fps_toggle: bool,
    hovered_backend_toggle: bool,
    hovered_wallpaper: bool,
    hovered_recent_wallpaper: Option<usize>,
    vsync_enabled: bool,
    debug_slot_backdrop_enabled: bool,
    has_backend_toggle: bool,
    recent_wallpapers: &[SidebarSavedWallpaperItem],
) {
    let panel_top = CHROME_HEIGHT_PX.min(height);
    fill_rect_region(
        pixels,
        width,
        height,
        [0, panel_top, width, height.saturating_sub(panel_top)],
        SIDEBAR_BG_COLOR,
    );

    for (idx, rect) in layout
        .nav_item_rects
        .into_iter()
        .take(visible_nav_items().len())
        .enumerate()
    {
        if hovered_nav_item == Some(idx) {
            fill_rounded_rect_region_aa(
                pixels,
                width,
                height,
                rect,
                SIDEBAR_ITEM_RADIUS_PX,
                SIDEBAR_HOVER_COLOR,
            );
        }
    }
    if DEBUG_SLOT_TOGGLE_ENABLED && (hovered_debug_slot_backdrop || debug_slot_backdrop_enabled) {
        fill_rounded_rect_region_aa(
            pixels,
            width,
            height,
            layout.debug_slot_backdrop_rect,
            SIDEBAR_ITEM_RADIUS_PX,
            SIDEBAR_HOVER_COLOR,
        );
    }
    if hovered_wallpaper {
        fill_rounded_rect_region_aa(
            pixels,
            width,
            height,
            layout.wallpaper_rect,
            SIDEBAR_ITEM_RADIUS_PX,
            SIDEBAR_HOVER_COLOR,
        );
    }
    if hovered_fps_toggle || vsync_enabled {
        fill_rounded_rect_region_aa(
            pixels,
            width,
            height,
            layout.fps_toggle_rect,
            SIDEBAR_ITEM_RADIUS_PX,
            SIDEBAR_HOVER_COLOR,
        );
    }
    if has_backend_toggle && hovered_backend_toggle {
        fill_rounded_rect_region_aa(
            pixels,
            width,
            height,
            layout.backend_toggle_rect,
            SIDEBAR_ITEM_RADIUS_PX,
            SIDEBAR_HOVER_COLOR,
        );
    }
    for (idx, rect) in layout.recent_wallpaper_rects.iter().enumerate() {
        let item = match recent_wallpapers.get(idx) {
            Some(item) => item,
            None => continue,
        };
        let hover_alpha = if hovered_recent_wallpaper == Some(idx) {
            34
        } else {
            16
        };
        fill_rounded_rect_region_aa(
            pixels,
            width,
            height,
            *rect,
            SIDEBAR_ITEM_RADIUS_PX,
            [255, 255, 255, hover_alpha],
        );
        let thumb_rect = inset_rect(*rect, 3);
        blit_cover_bilinear(
            pixels,
            width,
            height,
            thumb_rect,
            item.thumb_pixels.as_slice(),
            item.thumb_width.max(1),
            item.thumb_height.max(1),
        );
    }
}
