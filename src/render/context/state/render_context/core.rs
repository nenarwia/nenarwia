use crate::core::app_settings::GraphicsBackendPreference;
use crate::core::loader::AsyncLoader;
use crate::core::wallpaper::WallpaperLibrary;
use crate::render::cache::{CacheUniform, PageDirectory, PageTable, PhysicalCache};
use crate::render::context::document::ActiveDocumentState;
use crate::render::streaming::feedback::GpuFeedback;
use crate::render::ui::{
    BackdropBlurUi, CanvasContextMenuUi, CodecNoticeUi, SidebarSavedWallpaperItem, SidebarUi,
    WallpaperPreviewUi, WallpaperUi, WindowChromeUi,
};
use crate::render::{atlas::MultiTierAtlas, gpu::GpuState};
use crate::spatial::camera::CameraUniform;
use crate::spatial::navigation::ViewportState;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::keyboard::ModifiersState;
use winit::window::{ResizeDirection, Window};

use super::background::AppBackgroundState;
use super::interaction::{EmptySlotClickStamp, MediaItemClickStamp, PendingCanvasClick};
use crate::render::context::state::{
    CommittedViewState, DrawAssemblyState, FramePacingMode, QualityStats, Stage0Metrics,
    StreamingConfig, StreamingRuntimeState,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct FeedbackState {
    pub last_ready_frame: u64,
    pub overflow_last: bool,
    pub latency_last: u32,
    pub has_results: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct WindowedPlacement {
    pub position: Option<PhysicalPosition<i32>>,
    pub size: PhysicalSize<u32>,
}

#[derive(Clone, Copy, Debug)]
pub struct ClientResizeState {
    pub direction: ResizeDirection,
    pub start_cursor_screen: PhysicalPosition<i32>,
    pub start_window_position: PhysicalPosition<i32>,
    pub start_window_size: PhysicalSize<u32>,
}

pub struct RenderContext {
    pub gpu: GpuState,
    pub document: ActiveDocumentState,
    pub viewport: ViewportState,
    pub render_pipeline: wgpu::RenderPipeline,
    pub slot_backdrop_pipeline: wgpu::RenderPipeline,
    pub scene_color_texture: wgpu::Texture,
    pub scene_color_view: wgpu::TextureView,
    pub backdrop_blur_a_texture: wgpu::Texture,
    pub backdrop_blur_a_view: wgpu::TextureView,
    pub backdrop_blur_b_texture: wgpu::Texture,
    pub backdrop_blur_b_view: wgpu::TextureView,
    pub backdrop_blur_size: winit::dpi::PhysicalSize<u32>,

    pub camera_uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
    pub camera_bind_group: wgpu::BindGroup,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub diffuse_bind_group: wgpu::BindGroup,

    pub cache_uniform: CacheUniform,
    pub cache_uniform_buffer: wgpu::Buffer,

    pub atlas: MultiTierAtlas,

    pub tile_cache: PhysicalCache,
    pub page_table: PageTable,
    pub page_directory: PageDirectory,
    pub debug_slot_backdrop_enabled: bool,
    pub committed_view: CommittedViewState,
    pub draw_assembly: DrawAssemblyState,
    pub hovered_id: Option<u64>,
    pub selected_id: Option<u64>,
    pub pending_canvas_click: Option<PendingCanvasClick>,
    pub last_empty_slot_click: Option<EmptySlotClickStamp>,
    pub last_media_click: Option<MediaItemClickStamp>,

    pub loader: AsyncLoader,
    pub cursor_pos: Option<winit::dpi::PhysicalPosition<f64>>,
    pub keyboard_modifiers: ModifiersState,
    pub mouse_left_down: bool,
    pub pending_titlebar_drag_origin: Option<winit::dpi::PhysicalPosition<f64>>,
    pub active_client_resize: Option<ClientResizeState>,
    pub frame_pacing_mode: FramePacingMode,
    pub frame_count: u64,
    pub last_update_at: Option<Instant>,
    pub pending_redraw: bool,
    pub next_continuous_redraw_at: Option<Instant>,
    pub tabs: Vec<crate::render::context::document::CanvasTabState>,
    pub active_tab: usize,
    pub next_tab_id: u64,
    pub streaming_runtime: StreamingRuntimeState,
    pub quality_visible_since: HashMap<u64, u64>,
    pub quality_last_cleanup_frame: u64,
    pub streaming: StreamingConfig,

    pub last_vram_info: Option<crate::core::vram::VramInfo>,
    pub last_vram_budget_check_at: Option<Instant>,

    pub quality_stats: QualityStats,
    pub stage0_metrics: Stage0Metrics,

    pub window: Arc<Window>,
    pub windowed_placement: Option<WindowedPlacement>,
    pub window_fake_maximized: bool,
    pub window_was_maximized_before_fullscreen: bool,
    pub window_was_fake_maximized_before_fullscreen: bool,
    pub graphics_backend_preference: Option<GraphicsBackendPreference>,

    pub backdrop_blur: BackdropBlurUi,
    pub wallpaper_ui: WallpaperUi,
    pub wallpaper_preview_ui: WallpaperPreviewUi,
    pub wallpaper_library: WallpaperLibrary,
    pub recent_wallpapers: Vec<SidebarSavedWallpaperItem>,
    pub window_chrome: WindowChromeUi,
    pub sidebar_ui: SidebarUi,
    pub canvas_context_menu: CanvasContextMenuUi,
    pub codec_notice: CodecNoticeUi,

    pub gpu_feedback: Option<GpuFeedback>,

    pub feedback: FeedbackState,
    pub app_background: AppBackgroundState,
}
