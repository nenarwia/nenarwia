use crate::core::loader::disk_cache::{cache_root, library_root};
use crate::core::loader::processor::{
    decode_cache_byte_limit_value, decode_cache_item_limit_value, lod_cache_byte_limit_value,
    lod_cache_item_limit_value, max_decode_jobs_value, total_ram_gib_value,
};
use crate::core::loader::runtime_decode_enabled;

#[derive(Clone, Copy, Debug)]
pub(super) struct InitTuning {
    pub canvas_media_slot_cpu_budget_ms: f32,
    pub max_inflight_canvas_media_slots: usize,
    pub min_visible_previews_per_frame: usize,
    pub min_visible_previews_moving_per_frame: usize,
    pub max_preview_requests_moving_per_frame: usize,
    pub zoom_reset_settle_frames: u64,
    pub zoom_reset_cooldown_frames: u64,
    pub preview_soft_reset_pan_delta_px: f32,
    pub preview_soft_reset_cooldown_frames: u64,
    pub slot_interaction_off_visible: usize,
    pub slot_interaction_on_immediate_visible: usize,
    pub slot_interaction_on_delay_frames: u64,
    pub use_gpu_feedback: bool,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct PreviewCoverageWindow {
    pub min_outstanding: usize,
    pub max_outstanding: usize,
    pub upload_ema_seed: f32,
}

pub(super) fn log_runtime_environment() {
    let runtime_decode = runtime_decode_enabled();
    let gpu_resize_enabled = crate::core::color::gpu_resize_enabled_value();
    log::info!(
        "Canvas media slots runtime: decode={} tile_codec=rgba_lz4_tile",
        if runtime_decode { "ON" } else { "OFF" },
    );
    log::info!(
        "GPU slot resize: {} (env CANVAS_GPU_TILE_RESIZE=on|off|auto)",
        if gpu_resize_enabled { "AUTO/ON" } else { "OFF" }
    );
    let library_path = library_root();
    let runtime_path = cache_root();
    log::info!(
        "Cache roots: library={} runtime={}",
        library_path.to_string_lossy(),
        runtime_path.to_string_lossy()
    );
}

pub(super) fn load_tuning() -> InitTuning {
    let slot_interaction_off_visible = parse_slot_interaction_off_visible();
    let slot_interaction_on_immediate_visible =
        parse_slot_interaction_on_immediate_visible(slot_interaction_off_visible);

    InitTuning {
        canvas_media_slot_cpu_budget_ms: parse_canvas_media_slot_cpu_budget_ms(),
        max_inflight_canvas_media_slots: parse_max_inflight_canvas_media_slots(),
        min_visible_previews_per_frame: parse_min_visible_previews_per_frame(),
        min_visible_previews_moving_per_frame: parse_min_visible_previews_moving_per_frame(),
        max_preview_requests_moving_per_frame: parse_max_preview_requests_moving_per_frame(),
        zoom_reset_settle_frames: parse_zoom_reset_settle_frames(),
        zoom_reset_cooldown_frames: parse_zoom_reset_cooldown_frames(),
        preview_soft_reset_pan_delta_px: parse_preview_soft_reset_pan_delta_px(),
        preview_soft_reset_cooldown_frames: parse_preview_soft_reset_cooldown_frames(),
        slot_interaction_off_visible,
        slot_interaction_on_immediate_visible,
        slot_interaction_on_delay_frames: parse_slot_interaction_on_delay_frames(),
        use_gpu_feedback: false,
    }
}

pub(super) fn log_tuning_summary(tuning: InitTuning) {
    let max_decode_jobs = max_decode_jobs_value();
    let decode_cache_items = decode_cache_item_limit_value();
    let decode_cache_mb = decode_cache_byte_limit_value() as f32 / (1024.0 * 1024.0);
    let lod_cache_items = lod_cache_item_limit_value();
    let lod_cache_mb = lod_cache_byte_limit_value() as f32 / (1024.0 * 1024.0);
    let ram_msg = total_ram_gib_value()
        .map(|v| format!("{v:.1}GiB"))
        .unwrap_or_else(|| "unknown".to_string());
    log::info!(
        "Canvas media slot scheduling: cpu_budget_ms/ref60={:.2} max_inflight={} max_decode_jobs={} | RAM={} decode_cache={:.0}MiB/{} items lod_cache={:.0}MiB/{} items",
        tuning.canvas_media_slot_cpu_budget_ms,
        tuning.max_inflight_canvas_media_slots,
        max_decode_jobs,
        ram_msg,
        decode_cache_mb,
        decode_cache_items,
        lod_cache_mb,
        lod_cache_items,
    );
    log::info!(
        "Preview scheduling: min_visible_previews/ref60 idle={} moving={} | max_preview_requests_moving/ref60={}",
        tuning.min_visible_previews_per_frame,
        tuning.min_visible_previews_moving_per_frame,
        tuning.max_preview_requests_moving_per_frame,
    );
    log::info!(
        "Zoom reset coalescing: settle_frames={} cooldown_frames={}",
        tuning.zoom_reset_settle_frames,
        tuning.zoom_reset_cooldown_frames,
    );
    log::info!(
        "Preview soft reset: pan_delta_px={:.1} cooldown_frames={}",
        tuning.preview_soft_reset_pan_delta_px,
        tuning.preview_soft_reset_cooldown_frames,
    );
    log::info!(
        "Slot interaction gate: OFF if visible>{} | delayed ON in [{}..={}] for {} ref60 frames | immediate ON if visible<{}",
        tuning.slot_interaction_off_visible,
        tuning.slot_interaction_on_immediate_visible,
        tuning.slot_interaction_off_visible,
        tuning.slot_interaction_on_delay_frames,
        tuning.slot_interaction_on_immediate_visible,
    );
}

pub(super) fn preview_coverage_window(
    max_thumbs_per_frame: usize,
    min_visible_previews_per_frame: usize,
) -> PreviewCoverageWindow {
    let min_outstanding = parse_preview_coverage_outstanding_min(max_thumbs_per_frame);
    let max_outstanding =
        parse_preview_coverage_outstanding_max(min_outstanding, max_thumbs_per_frame);

    PreviewCoverageWindow {
        min_outstanding,
        max_outstanding,
        upload_ema_seed: min_visible_previews_per_frame as f32,
    }
}

fn parse_canvas_media_slot_cpu_budget_ms() -> f32 {
    let val = std::env::var("CANVAS_TILE_CPU_BUDGET_MS")
        .ok()
        .and_then(|v| v.parse::<f32>().ok());
    let default = 0.0;
    match val {
        Some(v) if v.is_finite() && v > 0.0 => v.clamp(0.5, 16.0),
        _ => default,
    }
}

fn parse_max_inflight_canvas_media_slots() -> usize {
    let val = std::env::var("CANVAS_MAX_INFLIGHT_TILES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok());
    if let Some(v) = val {
        return v.clamp(1, 64);
    }
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let half = (cpus / 2).max(1);
    half.clamp(4, 8)
}

fn parse_min_visible_previews_per_frame() -> usize {
    std::env::var("CANVAS_MIN_VISIBLE_PREVIEWS_PER_FRAME")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map(|v| v.clamp(1, 1024))
        .unwrap_or(12)
}

fn parse_min_visible_previews_moving_per_frame() -> usize {
    std::env::var("CANVAS_MIN_VISIBLE_PREVIEWS_MOVING_PER_FRAME")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map(|v| v.clamp(1, 1024))
        .unwrap_or(12)
}

fn parse_max_preview_requests_moving_per_frame() -> usize {
    std::env::var("CANVAS_MAX_PREVIEW_REQUESTS_MOVING")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map(|v| v.clamp(1, 1024))
        .unwrap_or(32)
}

fn parse_preview_coverage_outstanding_min(max_thumbs_per_frame: usize) -> usize {
    std::env::var("CANVAS_PREVIEW_COVERAGE_OUTSTANDING_MIN")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map(|v| v.clamp(32, 16384))
        .unwrap_or_else(|| (max_thumbs_per_frame.saturating_mul(4)).clamp(128, 2048))
}

fn parse_preview_coverage_outstanding_max(
    min_outstanding: usize,
    max_thumbs_per_frame: usize,
) -> usize {
    let default = (max_thumbs_per_frame.saturating_mul(20))
        .max(min_outstanding.saturating_mul(2))
        .clamp(256, 16384);
    std::env::var("CANVAS_PREVIEW_COVERAGE_OUTSTANDING_MAX")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map(|v| v.clamp(min_outstanding.max(64), 32768))
        .unwrap_or(default)
}

fn parse_zoom_reset_settle_frames() -> u64 {
    std::env::var("CANVAS_ZOOM_RESET_SETTLE_FRAMES")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(|v| v.clamp(1, 120))
        .unwrap_or(8)
}

fn parse_zoom_reset_cooldown_frames() -> u64 {
    std::env::var("CANVAS_ZOOM_RESET_COOLDOWN_FRAMES")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(|v| v.clamp(0, 240))
        .unwrap_or(12)
}

fn parse_preview_soft_reset_pan_delta_px() -> f32 {
    std::env::var("CANVAS_PREVIEW_SOFT_RESET_PAN_DELTA_PX")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .map(|v| v.clamp(16.0, 4096.0))
        .unwrap_or(160.0)
}

fn parse_preview_soft_reset_cooldown_frames() -> u64 {
    std::env::var("CANVAS_PREVIEW_SOFT_RESET_COOLDOWN_FRAMES")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(|v| v.clamp(0, 240))
        .unwrap_or(8)
}

fn parse_slot_interaction_off_visible() -> usize {
    std::env::var("CANVAS_SLOT_INTERACTION_OFF_VISIBLE")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map(|v| v.clamp(2, 100_000_000))
        .unwrap_or(13_000)
}

fn parse_slot_interaction_on_immediate_visible(off_visible: usize) -> usize {
    std::env::var("CANVAS_SLOT_INTERACTION_ON_IMMEDIATE_VISIBLE")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map(|v| v.clamp(1, off_visible.saturating_sub(1).max(1)))
        .unwrap_or(off_visible.saturating_sub(1_000).max(1))
}

fn parse_slot_interaction_on_delay_frames() -> u64 {
    std::env::var("CANVAS_SLOT_INTERACTION_ON_DELAY_FRAMES")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(|v| v.clamp(0, 600))
        .unwrap_or(24)
}
