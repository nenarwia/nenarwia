use winit::window::Window;

use crate::core::metrics;
use crate::core::profiler::Profiler;
use crate::render::atlas::ThumbTier;
use crate::render::cache::directory::region::PT_TEXTURE_SIZE;
use crate::render::context::RenderContext;
use crate::render::streaming::canvas_media_slots::calculator::media_world_size_to_pixels;
use crate::render::streaming::preview::{pick_enabled_tier, required_thumb_tier};

fn stage0_log_enabled() -> bool {
    use std::sync::OnceLock;
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        let val = std::env::var("CANVAS_STAGE0_LOG")
            .unwrap_or_default()
            .to_lowercase();
        matches!(val.as_str(), "1" | "true" | "yes" | "on")
    })
}

fn stage0_hw_vram_enabled() -> bool {
    use std::sync::OnceLock;
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        let val = std::env::var("CANVAS_STAGE0_HW_VRAM")
            .unwrap_or_default()
            .to_lowercase();
        matches!(val.as_str(), "1" | "true" | "yes" | "on")
    })
}

fn title_telemetry_enabled() -> bool {
    use std::sync::OnceLock;
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        let val = std::env::var("CANVAS_TITLE_TELEMETRY")
            .unwrap_or_default()
            .to_lowercase();
        matches!(val.as_str(), "1" | "true" | "yes" | "on")
    })
}

#[derive(Clone, Copy, Debug, Default)]
struct TierHistogram {
    counts: [u32; 5],
}

impl TierHistogram {
    fn add(&mut self, tier: ThumbTier) {
        self.counts[tier.index()] = self.counts[tier.index()].saturating_add(1);
    }

    fn loaded(&self) -> u32 {
        self.counts.iter().copied().sum()
    }

    fn min_px(&self) -> u32 {
        let tiers = [
            ThumbTier::Px32,
            ThumbTier::Px64,
            ThumbTier::Px128,
            ThumbTier::Px256,
            ThumbTier::Px512,
        ];
        for tier in tiers {
            if self.counts[tier.index()] > 0 {
                return tier.page_size();
            }
        }
        0
    }

    fn max_px(&self) -> u32 {
        let tiers = [
            ThumbTier::Px512,
            ThumbTier::Px256,
            ThumbTier::Px128,
            ThumbTier::Px64,
            ThumbTier::Px32,
        ];
        for tier in tiers {
            if self.counts[tier.index()] > 0 {
                return tier.page_size();
            }
        }
        0
    }

    fn avg_px(&self) -> f32 {
        let loaded = self.loaded();
        if loaded == 0 {
            return 0.0;
        }
        let tiers = [
            ThumbTier::Px32,
            ThumbTier::Px64,
            ThumbTier::Px128,
            ThumbTier::Px256,
            ThumbTier::Px512,
        ];
        let weighted_sum_px: u64 = tiers
            .iter()
            .map(|tier| self.counts[tier.index()] as u64 * tier.page_size() as u64)
            .sum();
        weighted_sum_px as f32 / loaded as f32
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct VisiblePreviewObservability {
    visible_total: u32,
    required: TierHistogram,
    resident: TierHistogram,
    resident_missing: u32,
    path_atlas: u32,
    path_tiles: u32,
    path_missing: u32,
}

fn collect_visible_preview_observability(ctx: &RenderContext) -> VisiblePreviewObservability {
    let mut obs = VisiblePreviewObservability {
        visible_total: ctx.committed_view.visible_items.len() as u32,
        ..VisiblePreviewObservability::default()
    };

    for item in ctx.committed_view.visible_items.iter().copied() {
        let Some(raw) = ctx.scene.all_items_raw.get(item.idx) else {
            obs.resident_missing = obs.resident_missing.saturating_add(1);
            obs.path_missing = obs.path_missing.saturating_add(1);
            continue;
        };

        let Some((obj_px_w, obj_px_h)) = media_world_size_to_pixels(ctx, item.idx) else {
            obs.resident_missing = obs.resident_missing.saturating_add(1);
            obs.path_missing = obs.path_missing.saturating_add(1);
            continue;
        };
        let max_px = obj_px_w.max(obj_px_h);
        let required = pick_enabled_tier(&ctx.atlas, required_thumb_tier(max_px));
        obs.required.add(required);

        let has_preview = raw.uv_region[2] > 0.0 && raw.uv_region[3] > 0.0;
        let has_tiles = raw.params[0] >= 0.0 && raw.params[2] > 0.0 && raw.params[3] > 0.0;

        if has_tiles {
            obs.path_tiles = obs.path_tiles.saturating_add(1);
        } else if has_preview {
            obs.path_atlas = obs.path_atlas.saturating_add(1);
        } else {
            obs.path_missing = obs.path_missing.saturating_add(1);
        }

        if !has_preview {
            obs.resident_missing = obs.resident_missing.saturating_add(1);
            continue;
        }

        if let Some(tier) = ThumbTier::decode_uv_x(raw.uv_region[0]) {
            obs.resident.add(tier);
        } else {
            obs.resident_missing = obs.resident_missing.saturating_add(1);
        }
    }

    obs
}

pub(super) fn update_frame_telemetry(
    window: &Window,
    ctx: &mut RenderContext,
    profiler: &mut Profiler,
) {
    if !profiler.tick() {
        return;
    }

    let stats = ctx.take_quality_stats();
    let fps = profiler.fps.max(1);
    let miss_avg = if stats.visible_total_tiles > 0 {
        stats.visible_missing_tiles as f32 / stats.visible_total_tiles as f32
    } else {
        0.0
    };
    let miss_max = stats.max_visible_missing_ratio;
    let under_vis_avg = if stats.visible_undersample_samples > 0 {
        stats.visible_undersample_sum / stats.visible_undersample_samples as f32
    } else {
        1.0
    };
    let under_vis_max = if stats.visible_undersample_samples > 0 {
        stats.visible_undersample_max
    } else {
        1.0
    };
    let ttfq_avg = if stats.ttfq_samples > 0 {
        stats.ttfq_frames_sum as f32 / stats.ttfq_samples as f32
    } else {
        0.0
    };
    let ttfq_max = stats.ttfq_frames_max as f32;
    let ttfq_avg_ms = (ttfq_avg / fps as f32) * 1000.0;
    let ttfq_max_ms = (ttfq_max / fps as f32) * 1000.0;
    let preview_cov_ratio = stats.visible_preview_coverage_ratio_last;
    let preview_cov_covered = stats.visible_preview_covered_last;
    let preview_cov_total = stats.visible_preview_total_last;
    let preview_full_cov_avg_frames = if stats.preview_full_coverage_samples > 0 {
        stats.preview_full_coverage_frames_sum as f32 / stats.preview_full_coverage_samples as f32
    } else {
        0.0
    };
    let preview_full_cov_max_frames = stats.preview_full_coverage_frames_max as f32;
    let preview_full_cov_avg_ms = (preview_full_cov_avg_frames / fps as f32) * 1000.0;
    let preview_full_cov_max_ms = (preview_full_cov_max_frames / fps as f32) * 1000.0;
    let vis_tiles = stats.visible_tiles_last;
    let cache_slots = ctx.tile_cache.total_slots;
    let scene_total = ctx.scene.total_count;
    let vis_items = stats.visible_items_last;
    let no_atlas = stats.visible_no_atlas_last;
    let tile_evict = stats.tile_evictions_last;
    let region_evict = stats.page_dir_evictions_last;
    let preview_evict = stats.preview_evictions_last;
    let preview_missing_any = stats.preview_missing_any_last;
    let preview_upgrade_needed = stats.preview_upgrade_needed_last;
    let preview_presence_gaps = stats.preview_presence_gaps_last;
    let preview_quality_phase_enabled = stats.preview_quality_phase_enabled_last;
    let preview_pending_cov = stats.preview_pending_coverage_last;
    let preview_pending_quality = stats.preview_pending_quality_last;
    let preview_pending_pruned = stats.preview_pending_pruned_last;
    let preview_pending_quality_dropped = stats.preview_pending_quality_dropped_last;
    let preview_upload_applied_cov = stats.preview_upload_applied_coverage_last;
    let preview_upload_applied_quality = stats.preview_upload_applied_quality_last;
    let preview_upload_drop_epoch = stats.preview_upload_dropped_epoch_last;
    let preview_upload_drop_not_pending = stats.preview_upload_dropped_not_pending_last;
    let preview_upload_drop_missing = stats.preview_upload_dropped_missing_last;
    let preview_upload_drop_no_slot = stats.preview_upload_dropped_no_slot_last;
    let fb_pages = stats.feedback_pages_last;
    let fb_over = stats.feedback_overflow_last;
    let fb_lat = stats.feedback_latency_last;
    let fb_mode = ctx
        .gpu_feedback
        .as_ref()
        .map(|fb| fb.mode_label())
        .unwrap_or("OFF");
    let vis_over = if cache_slots > 0 {
        vis_tiles as f32 / cache_slots as f32
    } else {
        0.0
    };

    let stage0 = ctx.take_stage0_metrics();
    let io = metrics::take_io_stats();
    let atlas_usage = ctx.atlas.usage();
    let requested_pages = ctx.streaming_runtime.canvas_media_slots.pending.len()
        + ctx.streaming_runtime.canvas_media_slots.queue_visible.len()
        + ctx
            .streaming_runtime
            .canvas_media_slots
            .queue_prefetch
            .len();
    let resident_pages = ctx.page_table.mapping.len();

    let tile_slot_bytes = ctx.tile_cache.bytes_per_tile();
    let tile_budget_bytes = ctx.tile_cache.bytes_per_tile() * (ctx.tile_cache.total_slots as u64);
    let tile_used_bytes = resident_pages as u64 * tile_slot_bytes;
    let page_dir_bytes = (PT_TEXTURE_SIZE as u64) * (PT_TEXTURE_SIZE as u64) * 4;
    let vram_budget_bytes = tile_budget_bytes
        .saturating_add(atlas_usage.total_bytes)
        .saturating_add(page_dir_bytes);
    let vram_used_est_bytes = tile_used_bytes
        .saturating_add(atlas_usage.used_bytes)
        .saturating_add(page_dir_bytes);

    let io_mib = io.bytes_read as f32 / (1024.0 * 1024.0);
    let budget_mib = vram_budget_bytes as f32 / (1024.0 * 1024.0);
    let used_mib = vram_used_est_bytes as f32 / (1024.0 * 1024.0);
    let atlas_used_pct = if atlas_usage.total_slots > 0 {
        (atlas_usage.used_slots as f32 / atlas_usage.total_slots as f32) * 100.0
    } else {
        0.0
    };
    let decode_ms_avg = if io.decode_jobs > 0 {
        io.decode_ms as f32 / io.decode_jobs as f32
    } else {
        0.0
    };
    let tiles_started_frame = io.tiles_started_per_frame;
    let tile_budget_hits = io.frame_budget_hit_count;
    let avg_tile_ms = io.avg_tile_build_ms;
    let hw_msg = if stage0_log_enabled() && stage0_hw_vram_enabled() {
        if let Some(v) = ctx.last_vram_info {
            format!(
                " | HW vram used={:.2}GiB free={:.2}GiB",
                v.used_bytes as f32 / (1024.0 * 1024.0 * 1024.0),
                v.free_gib()
            )
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    let decode_cache_mib = io.decode_cache_bytes as f32 / (1024.0 * 1024.0);
    let decode_cache_hits = io.decode_cache_hit;
    let decode_cache_miss = io.decode_cache_miss;
    let decode_cache_items = io.decode_cache_items;
    let stage0_frames = stage0.frames;
    let ddc_write_mib = io.ddc_write_bytes as f32 / (1024.0 * 1024.0);
    let ddc_write_avg_ms = if io.ddc_write_completed_count > 0 {
        io.ddc_write_ms as f32 / io.ddc_write_completed_count as f32
    } else {
        0.0
    };
    let thumb_job_ms_avg = if io.thumb_job_jobs > 0 {
        io.thumb_job_ms as f32 / io.thumb_job_jobs as f32
    } else {
        0.0
    };
    let tile_job_ms_avg = if io.tile_job_jobs > 0 {
        io.tile_job_ms as f32 / io.tile_job_jobs as f32
    } else {
        0.0
    };
    let jpeg_turbo_ms_avg = if io.jpeg_turbo_jobs > 0 {
        io.jpeg_turbo_ms as f32 / io.jpeg_turbo_jobs as f32
    } else {
        0.0
    };
    let gpu_resize_ms_avg = if io.gpu_resize_ok > 0 {
        io.gpu_resize_ms as f32 / io.gpu_resize_ok as f32
    } else {
        0.0
    };
    let cpu_resize_simd_ms_avg = if io.cpu_resize_simd_ok > 0 {
        io.cpu_resize_simd_ms as f32 / io.cpu_resize_simd_ok as f32
    } else {
        0.0
    };
    let thumb_preview_draft_jobs = io.thumb_preview_draft_jobs;
    let thumb_preview_medium_jobs = io.thumb_preview_medium_jobs;
    let thumb_preview_full_jobs = io
        .thumb_job_jobs
        .saturating_sub(thumb_preview_draft_jobs.saturating_add(thumb_preview_medium_jobs));
    let jpeg_shrink = if io.jpeg_decode_out_pixels > 0 {
        io.jpeg_decode_src_pixels as f32 / io.jpeg_decode_out_pixels as f32
    } else {
        1.0
    };
    let tile_dispatch_total = io.tile_dispatched_visible + io.tile_dispatched_prefetch;
    let tile_wait_avg_frames = if tile_dispatch_total > 0 {
        io.tile_dispatch_wait_frames_sum as f32 / tile_dispatch_total as f32
    } else {
        0.0
    };
    let preview_obs = collect_visible_preview_observability(ctx);
    let preview_loaded = preview_obs.resident.loaded();
    let preview_loaded_pct = if preview_obs.visible_total == 0 {
        100.0
    } else {
        (preview_loaded as f32 / preview_obs.visible_total as f32) * 100.0
    };
    let preview_min_px = preview_obs.resident.min_px();
    let preview_avg_px = preview_obs.resident.avg_px();
    let preview_max_px = preview_obs.resident.max_px();
    let required_min_px = preview_obs.required.min_px();
    let required_avg_px = preview_obs.required.avg_px();
    let required_max_px = preview_obs.required.max_px();
    let slot_gate_pending_elapsed = ctx
        .streaming_runtime
        .slot_interaction_gate
        .pending_on_elapsed_frames(std::time::Instant::now())
        .unwrap_or(0);

    if stage0_log_enabled() {
        log::info!(
            "Stage0 | scene_total={} vis_items={} req_pages={} res_pages={} evict_pages/s={} | VRAM budget={:.1}MiB used_est={:.1}MiB atlas_used={}/{} ({:.1}%) preview_unique_ids={} | IO read={:.1}MiB/s pages/s={} | CPU ms avg(fr={}): vis={:.2} sched={:.2} upload={:.2} decode={:.2} ({} jobs) | JobMs thumb={:.2} ({} jobs p95/p99={}/{} samp={} ovf={}) slots={:.2} ({} jobs p95/p99={}/{} samp={} ovf={}) | JPEG turbo ok/fb/scaled/full={} / {} / {} / {} builtin={} (scaled_req={}) shrink={:.2}x ms={:.2} ({} jobs p95/p99={}/{} samp={} ovf={}) | Queue enq v/p={}/{} drop v/p={}/{} disp v/p={}/{} off v/p={}/{} prune v/p={}/{} wait_f avg/p95/p99={:.1}/{}/{} samp={} ovf={} | DecodeCache hit={} miss={} items={} {:.1}MiB | DDC wr enq={} drop={} q={} {:.1}MiB avg_ms={:.2} p95/p99={}/{} samp={} ovf={} | Preview cov={}/{} ({:.1}%) full_ms avg/max={:.0}/{:.0} evict/s={} | PrevDbg miss_any/upg={} / {} gaps={} q_on={} pend c/q={} / {} prune={} drop_q={} up c/q={} / {} drop ep/np/ms/ns={} / {} / {} / {} | SlotBudget ms={:.2} inflight_max={} avg_slot_ms={:.2} started={} budget_hits={}{}",
            scene_total,
            vis_items,
            requested_pages,
            resident_pages,
            stage0.evicted_pages,
            budget_mib,
            used_mib,
            atlas_usage.used_slots,
            atlas_usage.total_slots,
            atlas_used_pct,
            atlas_usage.unique_ids,
            io_mib,
            io.page_reads,
            stage0_frames,
            stage0.visibility_ms_avg,
            stage0.scheduler_ms_avg,
            stage0.upload_ms_avg,
            decode_ms_avg,
            io.decode_jobs,
            thumb_job_ms_avg,
            io.thumb_job_jobs,
            io.thumb_job_p95_ms,
            io.thumb_job_p99_ms,
            io.thumb_job_samples,
            io.thumb_job_overflow,
            tile_job_ms_avg,
            io.tile_job_jobs,
            io.tile_job_p95_ms,
            io.tile_job_p99_ms,
            io.tile_job_samples,
            io.tile_job_overflow,
            io.jpeg_turbo_used,
            io.jpeg_turbo_fallback,
            io.jpeg_turbo_scaled,
            io.jpeg_turbo_full,
            io.jpeg_builtin_used,
            io.jpeg_builtin_scaled_req,
            jpeg_shrink,
            jpeg_turbo_ms_avg,
            io.jpeg_turbo_jobs,
            io.jpeg_turbo_p95_ms,
            io.jpeg_turbo_p99_ms,
            io.jpeg_turbo_samples,
            io.jpeg_turbo_overflow,
            io.tile_enqueued_visible,
            io.tile_enqueued_prefetch,
            io.tile_enqueue_drop_visible,
            io.tile_enqueue_drop_prefetch,
            io.tile_dispatched_visible,
            io.tile_dispatched_prefetch,
            io.tile_dispatched_visible_offscreen,
            io.tile_dispatched_prefetch_offscreen,
            io.tile_pruned_visible_stale,
            io.tile_pruned_prefetch_stale,
            tile_wait_avg_frames,
            io.tile_dispatch_wait_p95_frames,
            io.tile_dispatch_wait_p99_frames,
            io.tile_dispatch_wait_samples,
            io.tile_dispatch_wait_overflow,
            decode_cache_hits,
            decode_cache_miss,
            decode_cache_items,
            decode_cache_mib,
            io.ddc_write_enqueued_count,
            io.ddc_write_dropped_count,
            io.ddc_write_queue_len,
            ddc_write_mib,
            ddc_write_avg_ms,
            io.ddc_write_p95_ms,
            io.ddc_write_p99_ms,
            io.ddc_write_samples,
            io.ddc_write_overflow,
            preview_cov_covered,
            preview_cov_total,
            preview_cov_ratio * 100.0,
            preview_full_cov_avg_ms,
            preview_full_cov_max_ms,
            preview_evict,
            preview_missing_any,
            preview_upgrade_needed,
            preview_presence_gaps,
            preview_quality_phase_enabled,
            preview_pending_cov,
            preview_pending_quality,
            preview_pending_pruned,
            preview_pending_quality_dropped,
            preview_upload_applied_cov,
            preview_upload_applied_quality,
            preview_upload_drop_epoch,
            preview_upload_drop_not_pending,
            preview_upload_drop_missing,
            preview_upload_drop_no_slot,
            ctx.streaming.canvas_media_slot_cpu_budget_ms,
            ctx.streaming.max_inflight_canvas_media_slots,
            avg_tile_ms,
            tiles_started_frame,
            tile_budget_hits,
            hw_msg,
        );
        log::info!(
            "Stage0GpuResize | ok/fallback={} / {} attempts={} ms_avg={:.2} p95/p99={}/{} samp={} ovf={}",
            io.gpu_resize_ok,
            io.gpu_resize_fallback_cpu,
            io.gpu_resize_jobs,
            gpu_resize_ms_avg,
            io.gpu_resize_p95_ms,
            io.gpu_resize_p99_ms,
            io.gpu_resize_samples,
            io.gpu_resize_overflow,
        );
        log::info!(
            "Stage0CpuResizeSimd | ok/fallback={} / {} attempts={} ms_avg={:.2} p95/p99={}/{} samp={} ovf={}",
            io.cpu_resize_simd_ok,
            io.cpu_resize_simd_fallback,
            io.cpu_resize_simd_attempts,
            cpu_resize_simd_ms_avg,
            io.cpu_resize_simd_p95_ms,
            io.cpu_resize_simd_p99_ms,
            io.cpu_resize_simd_samples,
            io.cpu_resize_simd_overflow,
        );
        log::info!(
            "Stage0ThumbDecodeMode | draft/medium/full={} / {} / {} | motion_tier={} ema_px={:.1}",
            thumb_preview_draft_jobs,
            thumb_preview_medium_jobs,
            thumb_preview_full_jobs,
            ctx.viewport_runtime().preview_motion_tier.as_str(),
            ctx.viewport_runtime().preview_motion_px_ema,
        );
        log::info!(
            "Stage0PreviewQuality | loaded={}/{} ({:.1}%) no_preview={} | tier32/64/128/256/512={} / {} / {} / {} / {} | current_px min/avg/max={}/{:.1}/{}",
            preview_loaded,
            preview_obs.visible_total,
            preview_loaded_pct,
            preview_obs.resident_missing,
            preview_obs.resident.counts[ThumbTier::Px32.index()],
            preview_obs.resident.counts[ThumbTier::Px64.index()],
            preview_obs.resident.counts[ThumbTier::Px128.index()],
            preview_obs.resident.counts[ThumbTier::Px256.index()],
            preview_obs.resident.counts[ThumbTier::Px512.index()],
            preview_min_px,
            preview_avg_px,
            preview_max_px,
        );
        log::info!(
            "Stage0PreviewNow | required tier32/64/128/256/512={} / {} / {} / {} / {} req_px min/avg/max={}/{:.1}/{} | resident tier32/64/128/256/512={} / {} / {} / {} / {} res_px min/avg/max={}/{:.1}/{} | path atlas/tiles/missing={} / {} / {}",
            preview_obs.required.counts[ThumbTier::Px32.index()],
            preview_obs.required.counts[ThumbTier::Px64.index()],
            preview_obs.required.counts[ThumbTier::Px128.index()],
            preview_obs.required.counts[ThumbTier::Px256.index()],
            preview_obs.required.counts[ThumbTier::Px512.index()],
            required_min_px,
            required_avg_px,
            required_max_px,
            preview_obs.resident.counts[ThumbTier::Px32.index()],
            preview_obs.resident.counts[ThumbTier::Px64.index()],
            preview_obs.resident.counts[ThumbTier::Px128.index()],
            preview_obs.resident.counts[ThumbTier::Px256.index()],
            preview_obs.resident.counts[ThumbTier::Px512.index()],
            preview_min_px,
            preview_avg_px,
            preview_max_px,
            preview_obs.path_atlas,
            preview_obs.path_tiles,
            preview_obs.path_missing,
        );
        log::info!(
            "Stage0SlotGate | on={} vis_items={} off_thr={} on_immediate={} on_delay_frames={} pending_on_elapsed={}",
            if ctx.streaming_runtime.slot_interaction_gate.enabled { 1 } else { 0 },
            vis_items,
            ctx.streaming_runtime
                .slot_interaction_gate
                .off_visible_threshold,
            ctx.streaming_runtime
                .slot_interaction_gate
                .on_immediate_visible_threshold,
            ctx.streaming_runtime.slot_interaction_gate.on_delay_frames,
            slot_gate_pending_elapsed,
        );
    }

    if title_telemetry_enabled() {
        window.set_title(&format!(
            "nenarwia | FPS: {} | Objs: {} | Miss: {:>4.1}/{:>4.1}% | TTFQ: {:.0}/{:.0}ms | Preview: {}/{} ({:.0}%) full {:.0}/{:.0}ms evict={} | QClamp: {} | TierDown: {} | Under: {:.2}/{:.2} | UnderVis: {:.2}/{:.2} | FBmode={} FB: {} ovf={} lat={}f | VisSlots: {}/{} ({:.2}x) | NoAtlas: {}/{} | Evict: t{} r{} | SlotBudget: {:.1}ms inflight={} avg={:.2}ms start={}",
            profiler.fps,
            ctx.scene.total_count,
            miss_avg * 100.0,
            miss_max * 100.0,
            ttfq_avg_ms,
            ttfq_max_ms,
            preview_cov_covered,
            preview_cov_total,
            preview_cov_ratio * 100.0,
            preview_full_cov_avg_ms,
            preview_full_cov_max_ms,
            preview_evict,
            stats.lod_clamps,
            stats.tier_downgrades,
            stats.max_lod_undersample,
            stats.max_tier_undersample,
            under_vis_avg,
            under_vis_max,
            fb_mode,
            fb_pages,
            fb_over,
            fb_lat,
            vis_tiles,
            cache_slots,
            vis_over,
            no_atlas,
            vis_items,
            tile_evict,
            region_evict,
            ctx.streaming.canvas_media_slot_cpu_budget_ms,
            ctx.streaming.max_inflight_canvas_media_slots,
            avg_tile_ms,
            tiles_started_frame,
        ));
    }
}
