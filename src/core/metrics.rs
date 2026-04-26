use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

const LAT_SAMPLES_CAP: usize = 8192;

#[derive(Clone, Copy, Debug, Default)]
struct LatencySnapshot {
    p95_ms: u64,
    p99_ms: u64,
    samples: u64,
    overflow: u64,
}

#[derive(Debug)]
struct LatencySamples {
    samples: Vec<u16>,
    overflow: u64,
}

impl Default for LatencySamples {
    fn default() -> Self {
        Self {
            samples: Vec::with_capacity(LAT_SAMPLES_CAP),
            overflow: 0,
        }
    }
}

fn latency_store(slot: &OnceLock<Mutex<LatencySamples>>) -> &Mutex<LatencySamples> {
    slot.get_or_init(|| Mutex::new(LatencySamples::default()))
}

fn record_latency_ms(slot: &OnceLock<Mutex<LatencySamples>>, ms: u64) {
    if let Ok(mut data) = latency_store(slot).lock() {
        if data.samples.len() < LAT_SAMPLES_CAP {
            data.samples.push(ms.min(u16::MAX as u64) as u16);
        } else {
            data.overflow = data.overflow.saturating_add(1);
        }
    }
}

fn take_latency_snapshot(slot: &OnceLock<Mutex<LatencySamples>>) -> LatencySnapshot {
    let (mut samples, overflow) = if let Ok(mut data) = latency_store(slot).lock() {
        let samples = std::mem::replace(&mut data.samples, Vec::with_capacity(LAT_SAMPLES_CAP));
        let overflow = data.overflow;
        data.overflow = 0;
        (samples, overflow)
    } else {
        (Vec::new(), 0)
    };

    if samples.is_empty() {
        return LatencySnapshot {
            p95_ms: 0,
            p99_ms: 0,
            samples: 0,
            overflow,
        };
    }

    samples.sort_unstable();
    let n = samples.len();
    let i95 = ((n - 1) * 95) / 100;
    let i99 = ((n - 1) * 99) / 100;
    LatencySnapshot {
        p95_ms: samples[i95] as u64,
        p99_ms: samples[i99] as u64,
        samples: n as u64,
        overflow,
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct IoStats {
    pub bytes_read: u64,
    pub page_reads: u64,
    pub decode_ms: u64,
    pub decode_jobs: u64,
    pub decode_cache_hit: u64,
    pub decode_cache_miss: u64,
    pub decode_cache_bytes: u64,
    pub decode_cache_items: u64,
    pub avg_tile_build_ms: f32,
    pub tiles_started_per_frame: u64,
    pub frame_budget_hit_count: u64,
    pub ddc_write_enqueued_count: u64,
    pub ddc_write_dropped_count: u64,
    pub ddc_write_queue_len: u64,
    pub ddc_write_ms: u64,
    pub ddc_write_bytes: u64,
    pub ddc_write_completed_count: u64,
    pub ddc_write_p95_ms: u64,
    pub ddc_write_p99_ms: u64,
    pub ddc_write_samples: u64,
    pub ddc_write_overflow: u64,
    pub jpeg_turbo_used: u64,
    pub jpeg_turbo_fallback: u64,
    pub jpeg_turbo_scaled: u64,
    pub jpeg_turbo_full: u64,
    pub jpeg_builtin_used: u64,
    pub jpeg_builtin_scaled_req: u64,
    pub jpeg_decode_src_pixels: u64,
    pub jpeg_decode_out_pixels: u64,
    pub jpeg_turbo_ms: u64,
    pub jpeg_turbo_jobs: u64,
    pub jpeg_turbo_p95_ms: u64,
    pub jpeg_turbo_p99_ms: u64,
    pub jpeg_turbo_samples: u64,
    pub jpeg_turbo_overflow: u64,
    pub gpu_resize_jobs: u64,
    pub gpu_resize_ok: u64,
    pub gpu_resize_fallback_cpu: u64,
    pub gpu_resize_ms: u64,
    pub gpu_resize_p95_ms: u64,
    pub gpu_resize_p99_ms: u64,
    pub gpu_resize_samples: u64,
    pub gpu_resize_overflow: u64,
    pub cpu_resize_simd_attempts: u64,
    pub cpu_resize_simd_ok: u64,
    pub cpu_resize_simd_fallback: u64,
    pub cpu_resize_simd_ms: u64,
    pub cpu_resize_simd_p95_ms: u64,
    pub cpu_resize_simd_p99_ms: u64,
    pub cpu_resize_simd_samples: u64,
    pub cpu_resize_simd_overflow: u64,
    pub thumb_job_ms: u64,
    pub thumb_job_jobs: u64,
    pub thumb_job_p95_ms: u64,
    pub thumb_job_p99_ms: u64,
    pub thumb_job_samples: u64,
    pub thumb_job_overflow: u64,
    pub thumb_preview_draft_jobs: u64,
    pub thumb_preview_medium_jobs: u64,
    pub tile_job_ms: u64,
    pub tile_job_jobs: u64,
    pub tile_job_p95_ms: u64,
    pub tile_job_p99_ms: u64,
    pub tile_job_samples: u64,
    pub tile_job_overflow: u64,
    pub tile_enqueued_visible: u64,
    pub tile_enqueued_prefetch: u64,
    pub tile_enqueue_drop_visible: u64,
    pub tile_enqueue_drop_prefetch: u64,
    pub tile_dispatched_visible: u64,
    pub tile_dispatched_prefetch: u64,
    pub tile_dispatched_visible_offscreen: u64,
    pub tile_dispatched_prefetch_offscreen: u64,
    pub tile_pruned_visible_stale: u64,
    pub tile_pruned_prefetch_stale: u64,
    pub tile_dispatch_wait_frames_sum: u64,
    pub tile_dispatch_wait_p95_frames: u64,
    pub tile_dispatch_wait_p99_frames: u64,
    pub tile_dispatch_wait_samples: u64,
    pub tile_dispatch_wait_overflow: u64,
}

static IO_BYTES_READ: AtomicU64 = AtomicU64::new(0);
static IO_PAGE_READS: AtomicU64 = AtomicU64::new(0);
static DECODE_MS: AtomicU64 = AtomicU64::new(0);
static DECODE_JOBS: AtomicU64 = AtomicU64::new(0);
static DECODE_CACHE_HIT: AtomicU64 = AtomicU64::new(0);
static DECODE_CACHE_MISS: AtomicU64 = AtomicU64::new(0);
static DECODE_CACHE_BYTES: AtomicU64 = AtomicU64::new(0);
static DECODE_CACHE_ITEMS: AtomicU64 = AtomicU64::new(0);
static TILES_REQUESTED: AtomicU64 = AtomicU64::new(0);
static TILES_STARTED: AtomicU64 = AtomicU64::new(0);
static TILE_BUILD_MS_EMA_FP: AtomicU64 = AtomicU64::new(0);
static TILES_STARTED_PER_FRAME: AtomicU64 = AtomicU64::new(0);
static FRAME_BUDGET_HIT_COUNT: AtomicU64 = AtomicU64::new(0);
static TILES_SINGLEFLIGHT_HIT: AtomicU64 = AtomicU64::new(0);
static THUMBS_SINGLEFLIGHT_HIT: AtomicU64 = AtomicU64::new(0);
static DDC_WRITE_ENQUEUED_COUNT: AtomicU64 = AtomicU64::new(0);
static DDC_WRITE_DROPPED_COUNT: AtomicU64 = AtomicU64::new(0);
static DDC_WRITE_QUEUE_LEN: AtomicU64 = AtomicU64::new(0);
static DDC_WRITE_MS: AtomicU64 = AtomicU64::new(0);
static DDC_WRITE_BYTES: AtomicU64 = AtomicU64::new(0);
static DDC_WRITE_COMPLETED_COUNT: AtomicU64 = AtomicU64::new(0);
static JPEG_TURBO_USED: AtomicU64 = AtomicU64::new(0);
static JPEG_TURBO_FALLBACK: AtomicU64 = AtomicU64::new(0);
static JPEG_TURBO_SCALED: AtomicU64 = AtomicU64::new(0);
static JPEG_TURBO_FULL: AtomicU64 = AtomicU64::new(0);
static JPEG_BUILTIN_USED: AtomicU64 = AtomicU64::new(0);
static JPEG_BUILTIN_SCALED_REQ: AtomicU64 = AtomicU64::new(0);
static JPEG_DECODE_SRC_PIXELS: AtomicU64 = AtomicU64::new(0);
static JPEG_DECODE_OUT_PIXELS: AtomicU64 = AtomicU64::new(0);
static JPEG_TURBO_MS: AtomicU64 = AtomicU64::new(0);
static JPEG_TURBO_JOBS: AtomicU64 = AtomicU64::new(0);
static GPU_RESIZE_JOBS: AtomicU64 = AtomicU64::new(0);
static GPU_RESIZE_OK: AtomicU64 = AtomicU64::new(0);
static GPU_RESIZE_FALLBACK_CPU: AtomicU64 = AtomicU64::new(0);
static GPU_RESIZE_MS: AtomicU64 = AtomicU64::new(0);
static CPU_RESIZE_SIMD_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static CPU_RESIZE_SIMD_OK: AtomicU64 = AtomicU64::new(0);
static CPU_RESIZE_SIMD_FALLBACK: AtomicU64 = AtomicU64::new(0);
static CPU_RESIZE_SIMD_MS: AtomicU64 = AtomicU64::new(0);
static THUMB_JOB_MS: AtomicU64 = AtomicU64::new(0);
static THUMB_JOB_JOBS: AtomicU64 = AtomicU64::new(0);
static THUMB_PREVIEW_DRAFT_JOBS: AtomicU64 = AtomicU64::new(0);
static THUMB_PREVIEW_MEDIUM_JOBS: AtomicU64 = AtomicU64::new(0);
static TILE_JOB_MS: AtomicU64 = AtomicU64::new(0);
static TILE_JOB_JOBS: AtomicU64 = AtomicU64::new(0);
static TILE_ENQUEUED_VISIBLE: AtomicU64 = AtomicU64::new(0);
static TILE_ENQUEUED_PREFETCH: AtomicU64 = AtomicU64::new(0);
static TILE_ENQUEUE_DROP_VISIBLE: AtomicU64 = AtomicU64::new(0);
static TILE_ENQUEUE_DROP_PREFETCH: AtomicU64 = AtomicU64::new(0);
static TILE_DISPATCHED_VISIBLE: AtomicU64 = AtomicU64::new(0);
static TILE_DISPATCHED_PREFETCH: AtomicU64 = AtomicU64::new(0);
static TILE_DISPATCHED_VISIBLE_OFFSCREEN: AtomicU64 = AtomicU64::new(0);
static TILE_DISPATCHED_PREFETCH_OFFSCREEN: AtomicU64 = AtomicU64::new(0);
static TILE_PRUNED_VISIBLE_STALE: AtomicU64 = AtomicU64::new(0);
static TILE_PRUNED_PREFETCH_STALE: AtomicU64 = AtomicU64::new(0);
static TILE_DISPATCH_WAIT_FRAMES_SUM: AtomicU64 = AtomicU64::new(0);

static DDC_WRITE_LATENCY: OnceLock<Mutex<LatencySamples>> = OnceLock::new();
static JPEG_TURBO_LATENCY: OnceLock<Mutex<LatencySamples>> = OnceLock::new();
static GPU_RESIZE_LATENCY: OnceLock<Mutex<LatencySamples>> = OnceLock::new();
static CPU_RESIZE_SIMD_LATENCY: OnceLock<Mutex<LatencySamples>> = OnceLock::new();
static THUMB_JOB_LATENCY: OnceLock<Mutex<LatencySamples>> = OnceLock::new();
static TILE_JOB_LATENCY: OnceLock<Mutex<LatencySamples>> = OnceLock::new();
static TILE_DISPATCH_WAIT_LATENCY: OnceLock<Mutex<LatencySamples>> = OnceLock::new();

pub fn record_io_read(bytes: u64) {
    IO_BYTES_READ.fetch_add(bytes, Ordering::Relaxed);
}

pub fn record_page_read(count: u64) {
    IO_PAGE_READS.fetch_add(count, Ordering::Relaxed);
}

pub fn record_decode_ms(ms: u64) {
    DECODE_MS.fetch_add(ms, Ordering::Relaxed);
}

pub fn record_decode_job() {
    DECODE_JOBS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_decode_cache_hit() {
    DECODE_CACHE_HIT.fetch_add(1, Ordering::Relaxed);
}

pub fn record_decode_cache_miss() {
    DECODE_CACHE_MISS.fetch_add(1, Ordering::Relaxed);
}

pub fn set_decode_cache_bytes(bytes: u64) {
    DECODE_CACHE_BYTES.store(bytes, Ordering::Relaxed);
}

pub fn set_decode_cache_items(items: u64) {
    DECODE_CACHE_ITEMS.store(items, Ordering::Relaxed);
}

pub fn record_tiles_requested() {
    TILES_REQUESTED.fetch_add(1, Ordering::Relaxed);
}

pub fn record_tiles_started() {
    TILES_STARTED.fetch_add(1, Ordering::Relaxed);
}

pub fn record_tile_build_ms(ms: u64) {
    const ALPHA_NUM: u64 = 1;
    const ALPHA_DEN: u64 = 8;
    let sample_fp = ms.saturating_mul(1000);
    let mut cur = TILE_BUILD_MS_EMA_FP.load(Ordering::Relaxed);
    loop {
        let next = if cur == 0 {
            sample_fp
        } else {
            let decay = cur.saturating_mul(ALPHA_DEN - ALPHA_NUM) / ALPHA_DEN;
            let add = sample_fp.saturating_mul(ALPHA_NUM) / ALPHA_DEN;
            decay.saturating_add(add)
        };
        match TILE_BUILD_MS_EMA_FP.compare_exchange(cur, next, Ordering::AcqRel, Ordering::Relaxed)
        {
            Ok(_) => break,
            Err(v) => cur = v,
        }
    }
}

pub fn avg_tile_build_ms() -> f32 {
    let fp = TILE_BUILD_MS_EMA_FP.load(Ordering::Relaxed);
    (fp as f32) / 1000.0
}

pub fn set_tiles_started_per_frame(count: u64) {
    TILES_STARTED_PER_FRAME.store(count, Ordering::Relaxed);
}

pub fn record_frame_budget_hit() {
    FRAME_BUDGET_HIT_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn record_tiles_singleflight_hit() {
    TILES_SINGLEFLIGHT_HIT.fetch_add(1, Ordering::Relaxed);
}

pub fn record_thumbs_singleflight_hit() {
    THUMBS_SINGLEFLIGHT_HIT.fetch_add(1, Ordering::Relaxed);
}

pub fn record_jpeg_turbo_success(
    src_w: usize,
    src_h: usize,
    out_w: usize,
    out_h: usize,
    scaled: bool,
) {
    JPEG_TURBO_USED.fetch_add(1, Ordering::Relaxed);
    if scaled {
        JPEG_TURBO_SCALED.fetch_add(1, Ordering::Relaxed);
    } else {
        JPEG_TURBO_FULL.fetch_add(1, Ordering::Relaxed);
    }
    let src_px = (src_w as u64).saturating_mul(src_h as u64);
    let out_px = (out_w as u64).saturating_mul(out_h as u64);
    JPEG_DECODE_SRC_PIXELS.fetch_add(src_px, Ordering::Relaxed);
    JPEG_DECODE_OUT_PIXELS.fetch_add(out_px, Ordering::Relaxed);
}

pub fn record_jpeg_turbo_fallback() {
    JPEG_TURBO_FALLBACK.fetch_add(1, Ordering::Relaxed);
}

pub fn record_jpeg_builtin_used(scaled_req: bool) {
    JPEG_BUILTIN_USED.fetch_add(1, Ordering::Relaxed);
    if scaled_req {
        JPEG_BUILTIN_SCALED_REQ.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn record_jpeg_turbo_ms(ms: u64) {
    JPEG_TURBO_MS.fetch_add(ms, Ordering::Relaxed);
    JPEG_TURBO_JOBS.fetch_add(1, Ordering::Relaxed);
    record_latency_ms(&JPEG_TURBO_LATENCY, ms);
}

pub fn record_gpu_resize_job() {
    GPU_RESIZE_JOBS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_gpu_resize_ok(ms: u64) {
    GPU_RESIZE_OK.fetch_add(1, Ordering::Relaxed);
    GPU_RESIZE_MS.fetch_add(ms, Ordering::Relaxed);
    record_latency_ms(&GPU_RESIZE_LATENCY, ms);
}

pub fn record_gpu_resize_fallback_cpu() {
    GPU_RESIZE_FALLBACK_CPU.fetch_add(1, Ordering::Relaxed);
}

pub fn record_cpu_resize_simd_attempt() {
    CPU_RESIZE_SIMD_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_cpu_resize_simd_ok(ms: u64) {
    CPU_RESIZE_SIMD_OK.fetch_add(1, Ordering::Relaxed);
    CPU_RESIZE_SIMD_MS.fetch_add(ms, Ordering::Relaxed);
    record_latency_ms(&CPU_RESIZE_SIMD_LATENCY, ms);
}

pub fn record_cpu_resize_simd_fallback() {
    CPU_RESIZE_SIMD_FALLBACK.fetch_add(1, Ordering::Relaxed);
}

pub fn record_thumb_job_ms(ms: u64) {
    THUMB_JOB_MS.fetch_add(ms, Ordering::Relaxed);
    THUMB_JOB_JOBS.fetch_add(1, Ordering::Relaxed);
    record_latency_ms(&THUMB_JOB_LATENCY, ms);
}

pub fn record_thumb_preview_draft_job() {
    THUMB_PREVIEW_DRAFT_JOBS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_thumb_preview_medium_job() {
    THUMB_PREVIEW_MEDIUM_JOBS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_tile_job_ms(ms: u64) {
    TILE_JOB_MS.fetch_add(ms, Ordering::Relaxed);
    TILE_JOB_JOBS.fetch_add(1, Ordering::Relaxed);
    record_latency_ms(&TILE_JOB_LATENCY, ms);
}

pub fn record_tile_enqueued(is_prefetch: bool) {
    if is_prefetch {
        TILE_ENQUEUED_PREFETCH.fetch_add(1, Ordering::Relaxed);
    } else {
        TILE_ENQUEUED_VISIBLE.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn record_tile_enqueue_drop(is_prefetch: bool) {
    if is_prefetch {
        TILE_ENQUEUE_DROP_PREFETCH.fetch_add(1, Ordering::Relaxed);
    } else {
        TILE_ENQUEUE_DROP_VISIBLE.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn record_tile_dispatched(is_prefetch: bool, offscreen: bool, wait_frames: u64) {
    if is_prefetch {
        TILE_DISPATCHED_PREFETCH.fetch_add(1, Ordering::Relaxed);
        if offscreen {
            TILE_DISPATCHED_PREFETCH_OFFSCREEN.fetch_add(1, Ordering::Relaxed);
        }
    } else {
        TILE_DISPATCHED_VISIBLE.fetch_add(1, Ordering::Relaxed);
        if offscreen {
            TILE_DISPATCHED_VISIBLE_OFFSCREEN.fetch_add(1, Ordering::Relaxed);
        }
    }
    TILE_DISPATCH_WAIT_FRAMES_SUM.fetch_add(wait_frames, Ordering::Relaxed);
    record_latency_ms(&TILE_DISPATCH_WAIT_LATENCY, wait_frames);
}

pub fn record_tile_pruned_stale(is_prefetch: bool) {
    if is_prefetch {
        TILE_PRUNED_PREFETCH_STALE.fetch_add(1, Ordering::Relaxed);
    } else {
        TILE_PRUNED_VISIBLE_STALE.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn take_io_stats() -> IoStats {
    let ddc_lat = take_latency_snapshot(&DDC_WRITE_LATENCY);
    let jpeg_lat = take_latency_snapshot(&JPEG_TURBO_LATENCY);
    let gpu_resize_lat = take_latency_snapshot(&GPU_RESIZE_LATENCY);
    let cpu_resize_simd_lat = take_latency_snapshot(&CPU_RESIZE_SIMD_LATENCY);
    let thumb_lat = take_latency_snapshot(&THUMB_JOB_LATENCY);
    let tile_lat = take_latency_snapshot(&TILE_JOB_LATENCY);
    let tile_wait_lat = take_latency_snapshot(&TILE_DISPATCH_WAIT_LATENCY);
    IoStats {
        bytes_read: IO_BYTES_READ.swap(0, Ordering::Relaxed),
        page_reads: IO_PAGE_READS.swap(0, Ordering::Relaxed),
        decode_ms: DECODE_MS.swap(0, Ordering::Relaxed),
        decode_jobs: DECODE_JOBS.swap(0, Ordering::Relaxed),
        decode_cache_hit: DECODE_CACHE_HIT.swap(0, Ordering::Relaxed),
        decode_cache_miss: DECODE_CACHE_MISS.swap(0, Ordering::Relaxed),
        decode_cache_bytes: DECODE_CACHE_BYTES.load(Ordering::Relaxed),
        decode_cache_items: DECODE_CACHE_ITEMS.load(Ordering::Relaxed),
        avg_tile_build_ms: avg_tile_build_ms(),
        tiles_started_per_frame: TILES_STARTED_PER_FRAME.load(Ordering::Relaxed),
        frame_budget_hit_count: FRAME_BUDGET_HIT_COUNT.swap(0, Ordering::Relaxed),
        ddc_write_enqueued_count: DDC_WRITE_ENQUEUED_COUNT.swap(0, Ordering::Relaxed),
        ddc_write_dropped_count: DDC_WRITE_DROPPED_COUNT.swap(0, Ordering::Relaxed),
        ddc_write_queue_len: DDC_WRITE_QUEUE_LEN.load(Ordering::Relaxed),
        ddc_write_ms: DDC_WRITE_MS.swap(0, Ordering::Relaxed),
        ddc_write_bytes: DDC_WRITE_BYTES.swap(0, Ordering::Relaxed),
        ddc_write_completed_count: DDC_WRITE_COMPLETED_COUNT.swap(0, Ordering::Relaxed),
        ddc_write_p95_ms: ddc_lat.p95_ms,
        ddc_write_p99_ms: ddc_lat.p99_ms,
        ddc_write_samples: ddc_lat.samples,
        ddc_write_overflow: ddc_lat.overflow,
        jpeg_turbo_used: JPEG_TURBO_USED.swap(0, Ordering::Relaxed),
        jpeg_turbo_fallback: JPEG_TURBO_FALLBACK.swap(0, Ordering::Relaxed),
        jpeg_turbo_scaled: JPEG_TURBO_SCALED.swap(0, Ordering::Relaxed),
        jpeg_turbo_full: JPEG_TURBO_FULL.swap(0, Ordering::Relaxed),
        jpeg_builtin_used: JPEG_BUILTIN_USED.swap(0, Ordering::Relaxed),
        jpeg_builtin_scaled_req: JPEG_BUILTIN_SCALED_REQ.swap(0, Ordering::Relaxed),
        jpeg_decode_src_pixels: JPEG_DECODE_SRC_PIXELS.swap(0, Ordering::Relaxed),
        jpeg_decode_out_pixels: JPEG_DECODE_OUT_PIXELS.swap(0, Ordering::Relaxed),
        jpeg_turbo_ms: JPEG_TURBO_MS.swap(0, Ordering::Relaxed),
        jpeg_turbo_jobs: JPEG_TURBO_JOBS.swap(0, Ordering::Relaxed),
        jpeg_turbo_p95_ms: jpeg_lat.p95_ms,
        jpeg_turbo_p99_ms: jpeg_lat.p99_ms,
        jpeg_turbo_samples: jpeg_lat.samples,
        jpeg_turbo_overflow: jpeg_lat.overflow,
        gpu_resize_jobs: GPU_RESIZE_JOBS.swap(0, Ordering::Relaxed),
        gpu_resize_ok: GPU_RESIZE_OK.swap(0, Ordering::Relaxed),
        gpu_resize_fallback_cpu: GPU_RESIZE_FALLBACK_CPU.swap(0, Ordering::Relaxed),
        gpu_resize_ms: GPU_RESIZE_MS.swap(0, Ordering::Relaxed),
        gpu_resize_p95_ms: gpu_resize_lat.p95_ms,
        gpu_resize_p99_ms: gpu_resize_lat.p99_ms,
        gpu_resize_samples: gpu_resize_lat.samples,
        gpu_resize_overflow: gpu_resize_lat.overflow,
        cpu_resize_simd_attempts: CPU_RESIZE_SIMD_ATTEMPTS.swap(0, Ordering::Relaxed),
        cpu_resize_simd_ok: CPU_RESIZE_SIMD_OK.swap(0, Ordering::Relaxed),
        cpu_resize_simd_fallback: CPU_RESIZE_SIMD_FALLBACK.swap(0, Ordering::Relaxed),
        cpu_resize_simd_ms: CPU_RESIZE_SIMD_MS.swap(0, Ordering::Relaxed),
        cpu_resize_simd_p95_ms: cpu_resize_simd_lat.p95_ms,
        cpu_resize_simd_p99_ms: cpu_resize_simd_lat.p99_ms,
        cpu_resize_simd_samples: cpu_resize_simd_lat.samples,
        cpu_resize_simd_overflow: cpu_resize_simd_lat.overflow,
        thumb_job_ms: THUMB_JOB_MS.swap(0, Ordering::Relaxed),
        thumb_job_jobs: THUMB_JOB_JOBS.swap(0, Ordering::Relaxed),
        thumb_job_p95_ms: thumb_lat.p95_ms,
        thumb_job_p99_ms: thumb_lat.p99_ms,
        thumb_job_samples: thumb_lat.samples,
        thumb_job_overflow: thumb_lat.overflow,
        thumb_preview_draft_jobs: THUMB_PREVIEW_DRAFT_JOBS.swap(0, Ordering::Relaxed),
        thumb_preview_medium_jobs: THUMB_PREVIEW_MEDIUM_JOBS.swap(0, Ordering::Relaxed),
        tile_job_ms: TILE_JOB_MS.swap(0, Ordering::Relaxed),
        tile_job_jobs: TILE_JOB_JOBS.swap(0, Ordering::Relaxed),
        tile_job_p95_ms: tile_lat.p95_ms,
        tile_job_p99_ms: tile_lat.p99_ms,
        tile_job_samples: tile_lat.samples,
        tile_job_overflow: tile_lat.overflow,
        tile_enqueued_visible: TILE_ENQUEUED_VISIBLE.swap(0, Ordering::Relaxed),
        tile_enqueued_prefetch: TILE_ENQUEUED_PREFETCH.swap(0, Ordering::Relaxed),
        tile_enqueue_drop_visible: TILE_ENQUEUE_DROP_VISIBLE.swap(0, Ordering::Relaxed),
        tile_enqueue_drop_prefetch: TILE_ENQUEUE_DROP_PREFETCH.swap(0, Ordering::Relaxed),
        tile_dispatched_visible: TILE_DISPATCHED_VISIBLE.swap(0, Ordering::Relaxed),
        tile_dispatched_prefetch: TILE_DISPATCHED_PREFETCH.swap(0, Ordering::Relaxed),
        tile_dispatched_visible_offscreen: TILE_DISPATCHED_VISIBLE_OFFSCREEN
            .swap(0, Ordering::Relaxed),
        tile_dispatched_prefetch_offscreen: TILE_DISPATCHED_PREFETCH_OFFSCREEN
            .swap(0, Ordering::Relaxed),
        tile_pruned_visible_stale: TILE_PRUNED_VISIBLE_STALE.swap(0, Ordering::Relaxed),
        tile_pruned_prefetch_stale: TILE_PRUNED_PREFETCH_STALE.swap(0, Ordering::Relaxed),
        tile_dispatch_wait_frames_sum: TILE_DISPATCH_WAIT_FRAMES_SUM.swap(0, Ordering::Relaxed),
        tile_dispatch_wait_p95_frames: tile_wait_lat.p95_ms,
        tile_dispatch_wait_p99_frames: tile_wait_lat.p99_ms,
        tile_dispatch_wait_samples: tile_wait_lat.samples,
        tile_dispatch_wait_overflow: tile_wait_lat.overflow,
    }
}
