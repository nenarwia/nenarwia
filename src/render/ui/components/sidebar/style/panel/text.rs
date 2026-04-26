use ab_glyph::{Font, PxScale, ScaleFont};

use crate::core::app_settings::GraphicsBackendPreference;
use crate::core::process_memory::ProcessRamUsage;
use crate::render::ui::raster::{draw_text_line, TextLineParams};
use crate::render::ui::sidebar::nav::{visible_nav_items, SidebarNavItem};
use crate::render::ui::sidebar::state::SidebarSavedWallpaperItem;
use crate::render::ui::sidebar::style::constants::{
    SIDEBAR_TEXT_ALPHA_ACTIVE, SIDEBAR_TEXT_ALPHA_IDLE, SIDEBAR_TEXT_RGB,
};
use crate::render::ui::text::measure_text_width;
use crate::render::ui::{DEBUG_SLOT_TOGGLE_ENABLED, SIDEBAR_PADDING_X_PX};

use super::super::text_utils::fit_to_width;
use super::clear_canvas::draw_clear_canvas_item;
use super::layout::SidebarPanelLayout;

pub(in crate::render::ui::sidebar::style::panel) fn draw_panel_text(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    layout: &SidebarPanelLayout,
    font: &ab_glyph::FontArc,
    hovered_nav_item: Option<usize>,
    hovered_debug_slot_backdrop: bool,
    hovered_fps_toggle: bool,
    hovered_backend_toggle: bool,
    hovered_wallpaper: bool,
    hovered_recent_wallpaper: Option<usize>,
    active_nav_item: Option<usize>,
    active_wallpaper: bool,
    vsync_enabled: bool,
    graphics_backend_preference: Option<GraphicsBackendPreference>,
    debug_slot_backdrop_enabled: bool,
    clear_canvas_ram_usage: Option<ProcessRamUsage>,
    recent_wallpapers: &[SidebarSavedWallpaperItem],
) {
    let item_scale = sidebar_item_scale();
    let item_scaled = font.as_scaled(item_scale);
    let item_ascent = item_scaled.ascent();
    let item_line_height =
        (item_scaled.ascent() - item_scaled.descent() + item_scaled.line_gap()).max(1.0);
    for (idx, item) in visible_nav_items().iter().copied().enumerate() {
        let rect = layout.nav_item_rects[idx];
        let highlighted = hovered_nav_item == Some(idx) || active_nav_item == Some(idx);
        let alpha = if highlighted {
            SIDEBAR_TEXT_ALPHA_ACTIVE
        } else {
            SIDEBAR_TEXT_ALPHA_IDLE
        };
        if item == SidebarNavItem::ClearCanvas {
            draw_clear_canvas_item(
                pixels,
                width,
                height,
                rect,
                font,
                highlighted,
                clear_canvas_ram_usage,
            );
        } else {
            let baseline = rect_baseline(rect, item_line_height, item_ascent);
            let max_text_width = rect[2] as f32 - 20.0;
            let text = fit_to_width(item.label(), font, item_scale, max_text_width);
            draw_text_line(TextLineParams {
                pixels,
                width,
                height,
                font,
                scale: item_scale,
                x: rect[0] as f32 + 10.0,
                y: baseline,
                text: &text,
                color: [
                    SIDEBAR_TEXT_RGB[0],
                    SIDEBAR_TEXT_RGB[1],
                    SIDEBAR_TEXT_RGB[2],
                    alpha,
                ],
            });
        }
    }

    let fps_baseline = rect_baseline(layout.fps_toggle_rect, item_line_height, item_ascent);
    let backend_baseline = rect_baseline(layout.backend_toggle_rect, item_line_height, item_ascent);
    if DEBUG_SLOT_TOGGLE_ENABLED {
        let debug_baseline = rect_baseline(
            layout.debug_slot_backdrop_rect,
            item_line_height,
            item_ascent,
        );
        draw_stateful_footer_item(
            pixels,
            width,
            height,
            font,
            item_scale,
            layout.debug_slot_backdrop_rect,
            debug_baseline,
            "Debug Slots",
            if debug_slot_backdrop_enabled {
                "ON"
            } else {
                "OFF"
            },
            hovered_debug_slot_backdrop || debug_slot_backdrop_enabled,
        );
    }
    draw_stateful_footer_item(
        pixels,
        width,
        height,
        font,
        item_scale,
        layout.fps_toggle_rect,
        fps_baseline,
        "VSync",
        if vsync_enabled { "ON" } else { "OFF" },
        hovered_fps_toggle || vsync_enabled,
    );

    if let Some(preference) = graphics_backend_preference {
        draw_stateful_footer_item(
            pixels,
            width,
            height,
            font,
            item_scale,
            layout.backend_toggle_rect,
            backend_baseline,
            "Graphics API",
            preference.label(),
            hovered_backend_toggle,
        );
    }

    let wallpaper_baseline = rect_baseline(layout.wallpaper_rect, item_line_height, item_ascent);
    let wallpaper_text = fit_to_width(
        "Change Wallpaper",
        font,
        item_scale,
        layout.wallpaper_rect[2] as f32 - 20.0,
    );
    let wallpaper_highlighted = hovered_wallpaper || active_wallpaper;
    let wallpaper_alpha = if wallpaper_highlighted {
        SIDEBAR_TEXT_ALPHA_ACTIVE
    } else {
        SIDEBAR_TEXT_ALPHA_IDLE
    };
    draw_text_line(TextLineParams {
        pixels,
        width,
        height,
        font,
        scale: item_scale,
        x: layout.wallpaper_rect[0] as f32 + 10.0,
        y: wallpaper_baseline,
        text: &wallpaper_text,
        color: [
            SIDEBAR_TEXT_RGB[0],
            SIDEBAR_TEXT_RGB[1],
            SIDEBAR_TEXT_RGB[2],
            wallpaper_alpha,
        ],
    });

    let section_scale = sidebar_section_scale();
    let section_scaled = font.as_scaled(section_scale);
    let section_ascent = section_scaled.ascent();
    let section_line_height =
        (section_scaled.ascent() - section_scaled.descent() + section_scaled.line_gap()).max(1.0);
    let section_baseline = layout.recent_section_top as f32
        + ((16.0 - section_line_height) * 0.5).max(0.0)
        + section_ascent;
    draw_text_line(TextLineParams {
        pixels,
        width,
        height,
        font,
        scale: section_scale,
        x: SIDEBAR_PADDING_X_PX as f32 + 2.0,
        y: section_baseline,
        text: "Recent Wallpapers",
        color: [255, 255, 255, 148],
    });

    if layout.recent_wallpaper_rects.is_empty() {
        draw_text_line(TextLineParams {
            pixels,
            width,
            height,
            font,
            scale: section_scale,
            x: SIDEBAR_PADDING_X_PX as f32 + 2.0,
            y: section_baseline + 26.0,
            text: "No saved wallpapers yet",
            color: [255, 255, 255, 104],
        });
    } else {
        let badge_scale = PxScale::from(10.5);
        for (idx, rect) in layout.recent_wallpaper_rects.iter().enumerate() {
            if recent_wallpapers.get(idx).is_none() {
                continue;
            }
            if hovered_recent_wallpaper == Some(idx) {
                let label = "Edit";
                let text_width = measure_text_width(label, font, badge_scale);
                let x = rect[0] as f32 + rect[2] as f32 - text_width - 10.0;
                let y = rect[1] as f32 + 14.0;
                draw_text_line(TextLineParams {
                    pixels,
                    width,
                    height,
                    font,
                    scale: badge_scale,
                    x,
                    y,
                    text: label,
                    color: [255, 255, 255, 220],
                });
            }
        }
    }
}

fn rect_baseline(rect: [u32; 4], line_height: f32, ascent: f32) -> f32 {
    rect[1] as f32 + ((rect[3] as f32 - line_height) * 0.5).max(0.0) + ascent
}

fn draw_stateful_footer_item(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    font: &ab_glyph::FontArc,
    scale: PxScale,
    rect: [u32; 4],
    baseline: f32,
    label: &str,
    state: &str,
    highlighted: bool,
) {
    let state_width = measure_text_width(state, font, scale);
    let state_x = (rect[0] as f32 + rect[2] as f32 - 10.0 - state_width).max(rect[0] as f32 + 10.0);
    let label_max_width = (state_x - (rect[0] as f32 + 10.0) - 10.0).max(0.0);
    let fitted_label = fit_to_width(label, font, scale, label_max_width);
    let alpha = if highlighted {
        SIDEBAR_TEXT_ALPHA_ACTIVE
    } else {
        SIDEBAR_TEXT_ALPHA_IDLE
    };
    draw_text_line(TextLineParams {
        pixels,
        width,
        height,
        font,
        scale,
        x: rect[0] as f32 + 10.0,
        y: baseline,
        text: &fitted_label,
        color: [
            SIDEBAR_TEXT_RGB[0],
            SIDEBAR_TEXT_RGB[1],
            SIDEBAR_TEXT_RGB[2],
            alpha,
        ],
    });
    draw_text_line(TextLineParams {
        pixels,
        width,
        height,
        font,
        scale,
        x: state_x,
        y: baseline,
        text: state,
        color: [
            SIDEBAR_TEXT_RGB[0],
            SIDEBAR_TEXT_RGB[1],
            SIDEBAR_TEXT_RGB[2],
            SIDEBAR_TEXT_ALPHA_ACTIVE,
        ],
    });
}

fn sidebar_item_scale() -> PxScale {
    PxScale::from(16.0)
}

fn sidebar_section_scale() -> PxScale {
    PxScale::from(13.0)
}
