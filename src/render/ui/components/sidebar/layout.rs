use std::time::Instant;

use winit::dpi::PhysicalSize;

use crate::core::app_settings::GraphicsBackendPreference;
use crate::core::process_memory::query_process_ram_usage;

use super::super::{
    UiUpdatable, UiUpdateCtx, BURGER_BTN_LEFT_PX, BURGER_BTN_SIZE_PX, BURGER_BTN_TOP_PX,
    SIDEBAR_WIDTH_PX,
};
use super::style::{
    build_burger_texture, build_sidebar_texture, ease_in_out_cubic, write_rect_vertices,
};
use super::{SidebarSavedWallpaperItem, SidebarUi};

impl SidebarUi {
    fn refresh_clear_canvas_ram_usage(&mut self) {
        if !(self.target_open || self.open_t > 0.01) {
            return;
        }

        let now = Instant::now();
        let due = self
            .last_clear_canvas_ram_sample_at
            .map(|last| {
                now.saturating_duration_since(last) >= Self::clear_canvas_ram_sample_interval()
            })
            .unwrap_or(true);
        if !due {
            return;
        }

        self.last_clear_canvas_ram_sample_at = Some(now);
        let usage = query_process_ram_usage();
        if self.clear_canvas_ram_usage != usage {
            self.clear_canvas_ram_usage = usage;
            self.panel_texture_dirty = true;
        }
    }

    pub fn set_recent_wallpapers(&mut self, items: &[SidebarSavedWallpaperItem]) {
        if self.recent_wallpapers.as_slice() == items {
            return;
        }
        self.recent_wallpapers.clear();
        self.recent_wallpapers.extend_from_slice(items);
        if self
            .hovered_recent_wallpaper
            .is_some_and(|idx| idx >= self.recent_wallpapers.len())
        {
            self.hovered_recent_wallpaper = None;
        }
        self.panel_texture_dirty = true;
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_size: PhysicalSize<u32>,
        vsync_enabled: bool,
        graphics_backend_preference: Option<GraphicsBackendPreference>,
        debug_slot_backdrop_enabled: bool,
    ) {
        if surface_size.width == 0 || surface_size.height == 0 {
            return;
        }

        self.refresh_clear_canvas_ram_usage();

        if self.vsync_enabled != vsync_enabled {
            self.vsync_enabled = vsync_enabled;
            self.panel_texture_dirty = true;
        }
        if self.graphics_backend_preference != graphics_backend_preference {
            self.graphics_backend_preference = graphics_backend_preference;
            self.panel_texture_dirty = true;
        }
        if self.debug_slot_backdrop_enabled != debug_slot_backdrop_enabled {
            self.debug_slot_backdrop_enabled = debug_slot_backdrop_enabled;
            self.panel_texture_dirty = true;
        }

        if !self.burger_texture_ready || self.burger_texture_dirty {
            let burger = build_burger_texture(self.hovered_burger);
            self.update_burger_texture(device, queue, &burger);
            self.burger_texture_ready = true;
            self.burger_texture_dirty = false;
        }

        if self.last_surface_height != surface_size.height
            || self.panel_tex_width != SIDEBAR_WIDTH_PX
            || self.panel_texture_dirty
        {
            let panel = build_sidebar_texture(
                surface_size.height,
                self.hovered_nav_item,
                self.hovered_debug_slot_backdrop,
                self.hovered_fps_toggle,
                self.hovered_backend_toggle,
                self.hovered_wallpaper,
                self.hovered_recent_wallpaper,
                self.active_nav_item,
                self.active_wallpaper,
                self.vsync_enabled,
                self.graphics_backend_preference,
                self.debug_slot_backdrop_enabled,
                self.clear_canvas_ram_usage,
                self.recent_wallpapers.as_slice(),
            );
            self.update_panel_texture(device, queue, &panel);
            self.nav_item_rects_local = panel
                .nav_item_rects
                .map(|r| [r[0] as f32, r[1] as f32, r[2] as f32, r[3] as f32]);
            self.debug_slot_backdrop_rect_local = [
                panel.debug_slot_backdrop_rect[0] as f32,
                panel.debug_slot_backdrop_rect[1] as f32,
                panel.debug_slot_backdrop_rect[2] as f32,
                panel.debug_slot_backdrop_rect[3] as f32,
            ];
            self.fps_toggle_rect_local = [
                panel.fps_toggle_rect[0] as f32,
                panel.fps_toggle_rect[1] as f32,
                panel.fps_toggle_rect[2] as f32,
                panel.fps_toggle_rect[3] as f32,
            ];
            self.backend_toggle_rect_local = [
                panel.backend_toggle_rect[0] as f32,
                panel.backend_toggle_rect[1] as f32,
                panel.backend_toggle_rect[2] as f32,
                panel.backend_toggle_rect[3] as f32,
            ];
            self.wallpaper_rect_local = [
                panel.wallpaper_rect[0] as f32,
                panel.wallpaper_rect[1] as f32,
                panel.wallpaper_rect[2] as f32,
                panel.wallpaper_rect[3] as f32,
            ];
            self.recent_wallpaper_rects_local = panel
                .recent_wallpaper_rects
                .iter()
                .map(|rect| {
                    [
                        rect[0] as f32,
                        rect[1] as f32,
                        rect[2] as f32,
                        rect[3] as f32,
                    ]
                })
                .collect();
            self.last_surface_height = surface_size.height;
            self.panel_texture_dirty = false;
        }

        self.advance_animation();

        let panel_x = -(SIDEBAR_WIDTH_PX as f32) * (1.0 - self.open_t);
        self.panel_rect_px = [
            panel_x,
            0.0,
            SIDEBAR_WIDTH_PX as f32,
            surface_size.height as f32,
        ];
        self.burger_rect_px = [
            BURGER_BTN_LEFT_PX as f32,
            BURGER_BTN_TOP_PX as f32,
            BURGER_BTN_SIZE_PX as f32,
            BURGER_BTN_SIZE_PX as f32,
        ];

        write_rect_vertices(
            queue,
            &self.panel_vertex_buffer,
            surface_size,
            self.panel_rect_px,
        );
        write_rect_vertices(
            queue,
            &self.burger_vertex_buffer,
            surface_size,
            self.burger_rect_px,
        );
    }

    pub fn is_animating(&self) -> bool {
        self.anim_started_at.is_some()
    }

    pub fn blur_width_px(&self) -> f32 {
        SIDEBAR_WIDTH_PX as f32 * self.open_t
    }

    pub(super) fn set_open(&mut self, open: bool) -> bool {
        let target_t = if open { 1.0 } else { 0.0 };
        if !open
            && (self.hovered_nav_item.is_some()
                || self.hovered_debug_slot_backdrop
                || self.hovered_fps_toggle
                || self.hovered_backend_toggle
                || self.hovered_wallpaper
                || self.hovered_recent_wallpaper.is_some())
        {
            self.hovered_nav_item = None;
            self.hovered_debug_slot_backdrop = false;
            self.hovered_fps_toggle = false;
            self.hovered_backend_toggle = false;
            self.hovered_wallpaper = false;
            self.hovered_recent_wallpaper = None;
            self.panel_texture_dirty = true;
        }
        if (self.open_t - target_t).abs() <= 0.0001 && self.anim_started_at.is_none() {
            self.target_open = open;
            self.anim_to_t = target_t;
            self.anim_from_t = target_t;
            return false;
        }
        self.target_open = open;
        self.anim_from_t = self.open_t;
        self.anim_to_t = target_t;
        self.anim_started_at = Some(std::time::Instant::now());
        true
    }

    fn advance_animation(&mut self) {
        let Some(started_at) = self.anim_started_at else {
            self.open_t = if self.target_open { 1.0 } else { 0.0 };
            return;
        };

        let duration_s = self.anim_duration.as_secs_f32().max(0.0001);
        let elapsed_s = std::time::Instant::now()
            .saturating_duration_since(started_at)
            .as_secs_f32();
        let t = (elapsed_s / duration_s).clamp(0.0, 1.0);
        let eased = ease_in_out_cubic(t);
        self.open_t = self.anim_from_t + (self.anim_to_t - self.anim_from_t) * eased;

        if t >= 1.0 {
            self.open_t = self.anim_to_t;
            self.anim_started_at = None;
        }
    }
}

impl UiUpdatable for SidebarUi {
    fn update_ui(&mut self, ctx: UiUpdateCtx<'_>) {
        self.update(
            ctx.device,
            ctx.queue,
            ctx.surface_size,
            ctx.vsync_enabled,
            ctx.graphics_backend_preference,
            ctx.debug_slot_backdrop_enabled,
        );
    }
}
