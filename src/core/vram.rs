/// VRAM probing utilities.
///
/// Goal: adapt caches to *current* GPU memory pressure.
/// On NVIDIA we can read total/used/free via NVML.
/// If anything fails (no NVIDIA / no permission / no NVML), we fall back to None.
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct VramInfo {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
}

impl VramInfo {
    pub fn free_gib(&self) -> f32 {
        self.free_bytes as f32 / (1024.0 * 1024.0 * 1024.0)
    }
    pub fn total_gib(&self) -> f32 {
        self.total_bytes as f32 / (1024.0 * 1024.0 * 1024.0)
    }
}

enum NvmlBackend {
    Ready(nvml_wrapper::Nvml),
    Unavailable,
}

#[derive(Clone, Copy, Debug)]
struct VramCacheEntry {
    at: Instant,
    value: Option<VramInfo>,
}

const VRAM_CACHE_TTL: Duration = Duration::from_millis(250);

fn backend() -> &'static Mutex<NvmlBackend> {
    static BACKEND: OnceLock<Mutex<NvmlBackend>> = OnceLock::new();
    BACKEND.get_or_init(|| {
        let backend = match nvml_wrapper::Nvml::init() {
            Ok(nvml) => NvmlBackend::Ready(nvml),
            Err(_) => NvmlBackend::Unavailable,
        };
        Mutex::new(backend)
    })
}

fn cache_entry() -> &'static Mutex<Option<VramCacheEntry>> {
    static CACHE: OnceLock<Mutex<Option<VramCacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

/// Try to read VRAM stats.
///
/// NOTE: We intentionally keep this best-effort. The engine must keep running
/// even when VRAM probing is unavailable.
pub fn query_vram() -> Option<VramInfo> {
    if let Ok(cache) = cache_entry().lock() {
        if let Some(entry) = *cache {
            if entry.at.elapsed() <= VRAM_CACHE_TTL {
                return entry.value;
            }
        }
    }

    let value = query_vram_nvml();
    if let Ok(mut cache) = cache_entry().lock() {
        *cache = Some(VramCacheEntry {
            at: Instant::now(),
            value,
        });
    }
    value
}

fn query_vram_nvml() -> Option<VramInfo> {
    // Initialize NVML once per process, then reuse the handle.
    let lock = backend();
    let guard = lock.lock().ok()?;
    let nvml = match &*guard {
        NvmlBackend::Ready(v) => v,
        NvmlBackend::Unavailable => return None,
    };

    // Best effort: first GPU.
    // For multi-GPU setups we'd ideally match wgpu adapter, but for now this is
    // good enough (and typical desktop setups have a single discrete GPU).
    let device = nvml.device_by_index(0).ok()?;
    let mem = device.memory_info().ok()?;

    Some(VramInfo {
        total_bytes: mem.total,
        used_bytes: mem.used,
        free_bytes: mem.free,
    })
}
