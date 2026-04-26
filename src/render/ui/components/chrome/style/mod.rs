mod constants;
mod controls;
mod geometry;
mod primitives;
mod tab_paint;
mod tab_strip;
mod text_utils;

use super::state::{ChromeTabView, ChromeTexture};
use controls::paint_chrome_base;
use tab_strip::build_tab_strip;

pub(super) fn build_chrome_texture(
    width: u32,
    _maximized: bool,
    tabs: &[ChromeTabView],
    active_tab: usize,
    hovered_tab: Option<usize>,
    hovered_close_tab: Option<usize>,
    hovered_add_tab: bool,
) -> ChromeTexture {
    let height = super::super::CHROME_HEIGHT_PX;
    let mut pixels = vec![0u8; width as usize * height as usize * 4];

    let controls = paint_chrome_base(&mut pixels, width, height);
    let tab_strip = build_tab_strip(
        &mut pixels,
        width,
        height,
        tabs,
        active_tab,
        hovered_tab,
        hovered_close_tab,
        hovered_add_tab,
        controls.controls_cluster_end,
        controls.drag_end,
    );

    ChromeTexture {
        pixels,
        width,
        height,
        close_rect: controls.close_rect,
        minimize_rect: controls.minimize_rect,
        maximize_rect: controls.maximize_rect,
        tab_indices: tab_strip.tab_indices,
        tab_rects: tab_strip.tab_rects,
        tab_close_rects: tab_strip.tab_close_rects,
        add_tab_rect: tab_strip.add_tab_rect,
        drag_rect: tab_strip.drag_rect,
    }
}
