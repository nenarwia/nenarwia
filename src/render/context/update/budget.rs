use std::sync::mpsc::{self, TryRecvError};
use std::time::Instant;

use crate::core::vram;
use crate::render::cache::{CacheUniform, PageTable, PhysicalCache};
use crate::render::context::budget::{self, CacheConfig};
use crate::render::context::state::RenderContext;
use crate::render::streaming::feedback::GpuFeedback;

const MIN_FREE_DELTA_BYTES: u64 = 128 * 1024 * 1024; // 128MiB

pub fn maybe_update_budget(ctx: &mut RenderContext) {
    poll_vram_budget_result(ctx);
    maybe_start_vram_budget_query(ctx);
}

fn maybe_start_vram_budget_query(ctx: &mut RenderContext) {
    let now = Instant::now();
    let check_interval = RenderContext::duration_for_reference_frames(60);
    let Some(last_check) = ctx.last_vram_budget_check_at else {
        ctx.last_vram_budget_check_at = Some(now);
        return;
    };
    if now.saturating_duration_since(last_check) < check_interval
        || ctx.app_background.vram_budget_rx.is_some()
    {
        return;
    }

    ctx.last_vram_budget_check_at = Some(now);
    let (tx, rx) = mpsc::channel();
    match std::thread::Builder::new()
        .name("vram-budget-probe".to_string())
        .spawn(move || {
            let _ = tx.send(vram::query_vram());
        }) {
        Ok(_) => {
            ctx.app_background.vram_budget_rx = Some(rx);
        }
        Err(err) => {
            log::warn!("Failed to start VRAM budget probe worker: {err}");
        }
    }
}

fn poll_vram_budget_result(ctx: &mut RenderContext) {
    let Some(rx) = ctx.app_background.vram_budget_rx.take() else {
        return;
    };

    match rx.try_recv() {
        Ok(Some(v)) => apply_vram_budget_sample(ctx, v),
        Ok(None) | Err(TryRecvError::Disconnected) => {}
        Err(TryRecvError::Empty) => {
            ctx.app_background.vram_budget_rx = Some(rx);
        }
    }
}

fn apply_vram_budget_sample(ctx: &mut RenderContext, v: vram::VramInfo) {
    if let Some(last) = ctx.last_vram_info {
        let delta = v.free_bytes.abs_diff(last.free_bytes);
        if delta < MIN_FREE_DELTA_BYTES {
            return;
        }
    }

    ctx.last_vram_info = Some(v);

    let max_dim = ctx.gpu.device.limits().max_texture_dimension_2d;
    let desired = budget::decide_cache_config(Some(v), max_dim);

    ctx.streaming.prefetch_radius_tiles = desired.prefetch_radius_tiles;
    ctx.streaming.max_canvas_media_slot_requests_per_frame =
        desired.max_canvas_media_slot_requests_per_frame;
    ctx.streaming.max_thumb_requests_per_frame = desired.max_thumb_requests_per_frame;

    // Do not rebuild preview atlases at runtime.
    // Runtime atlas rebuild clears UVs and causes visible flicker/pop-in.
    // Preview atlas sizing is fixed per run and changed only on next launch.
    let need_tiles = ctx.tile_cache.cache_dim != desired.tile_cache_dim;

    if !need_tiles {
        return;
    }

    let shrink_only = true;
    if shrink_only {
        if desired.tile_cache_dim > ctx.tile_cache.cache_dim {
            return;
        }
    }

    log::warn!(
        "VRAM pressure change detected -> rebuilding tile cache only: tile {}->{} | free {:.2}GiB",
        ctx.tile_cache.cache_dim,
        desired.tile_cache_dim,
        v.free_gib()
    );

    apply_cache_config(ctx, desired);
}

fn apply_cache_config(ctx: &mut RenderContext, cfg: CacheConfig) {
    let max_dim = ctx.gpu.device.limits().max_texture_dimension_2d;

    // Rebuild physical tile cache
    if ctx.tile_cache.cache_dim != cfg.tile_cache_dim {
        ctx.tile_cache = PhysicalCache::new(
            &ctx.gpu.device,
            cfg.tile_cache_dim,
            max_dim,
            ctx.gpu.tile_format,
        );
        ctx.page_table = PageTable::new(ctx.tile_cache.total_slots);

        ctx.cache_uniform = CacheUniform::new(ctx.tile_cache.cols);
        ctx.gpu.queue.write_buffer(
            &ctx.cache_uniform_buffer,
            0,
            bytemuck::cast_slice(&[ctx.cache_uniform]),
        );

        ctx.page_directory.reset_all(&ctx.gpu.queue);
        ctx.streaming_runtime.clear_canvas_media_slot_work();
    }

    // Refresh bind group
    ctx.diffuse_bind_group = ctx
        .gpu
        .device
        .create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &ctx.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(ctx.atlas.views()[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(ctx.atlas.views()[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(ctx.atlas.views()[2]),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(ctx.atlas.views()[3]),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(ctx.atlas.views()[4]),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(ctx.atlas.sampler_linear()),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(ctx.atlas.sampler_nearest()),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(&ctx.tile_cache.view),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::TextureView(&ctx.page_directory.view),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: ctx.cache_uniform_buffer.as_entire_binding(),
                },
            ],
            label: Some("diffuse_bind_group_rebuilt"),
        });

    if ctx.streaming.use_gpu_feedback {
        ctx.gpu_feedback = Some(GpuFeedback::new(
            &ctx.gpu.device,
            &ctx.camera_bind_group_layout,
            ctx.gpu.feedback_rt_supported,
        ));
        if let Some(fb) = ctx.gpu_feedback.as_ref() {
            ctx.draw_assembly.feedback_instance_bind_group = fb.create_instance_bind_group(
                &ctx.gpu.device,
                &ctx.draw_assembly.feedback_instance_buffer,
            );
            ctx.draw_assembly.feedback_collect_buf_bind_group = fb.create_collect_buf_bind_group(
                &ctx.gpu.device,
                &ctx.draw_assembly.feedback_instance_buffer,
            );
        }
        ctx.feedback.has_results = false;
        ctx.feedback.last_ready_frame = 0;
        ctx.feedback.overflow_last = false;
        ctx.feedback.latency_last = 0;
    } else {
        ctx.gpu_feedback = None;
    }

    // Apply streaming knobs
    ctx.streaming.prefetch_radius_tiles = cfg.prefetch_radius_tiles;
    ctx.streaming.max_canvas_media_slot_requests_per_frame =
        cfg.max_canvas_media_slot_requests_per_frame;
    ctx.streaming.max_thumb_requests_per_frame = cfg.max_thumb_requests_per_frame;

    ctx.mark_redraw_pending();
}
