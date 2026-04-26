use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use std::time::UNIX_EPOCH;

static ALLOW_RUNTIME_DECODE: OnceLock<bool> = OnceLock::new();
static MAX_DECODE_JOBS: OnceLock<usize> = OnceLock::new();
static TOTAL_RAM_BYTES: OnceLock<Option<u64>> = OnceLock::new();

const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * 1024 * 1024;
const MIN_CACHE_MB: u64 = 128;
const MAX_CACHE_MB: u64 = 4096;

fn total_ram_bytes() -> Option<u64> {
    *TOTAL_RAM_BYTES.get_or_init(query_total_ram_bytes)
}

#[cfg(target_os = "windows")]
fn query_total_ram_bytes() -> Option<u64> {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    let mut mem = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    if unsafe { GlobalMemoryStatusEx(&mut mem) }.is_ok() {
        Some(mem.ullTotalPhys)
    } else {
        None
    }
}

#[cfg(not(target_os = "windows"))]
fn query_total_ram_bytes() -> Option<u64> {
    None
}

fn clamp_cache_mb(mb: u64) -> u64 {
    mb.clamp(MIN_CACHE_MB, MAX_CACHE_MB)
}

fn auto_decode_cache_mb() -> Option<u64> {
    let total = total_ram_bytes()?;
    Some(clamp_cache_mb(total.saturating_mul(3) / 100 / MIB))
}

fn auto_lod_cache_mb() -> Option<u64> {
    let total = total_ram_bytes()?;
    Some(clamp_cache_mb(total.saturating_mul(2) / 100 / MIB))
}

fn auto_decode_cache_items(cache_bytes: u64) -> usize {
    // Approximate one full decoded image entry as ~64MiB.
    let approx_item_bytes = 64 * MIB;
    let items = (cache_bytes / approx_item_bytes).max(1) as usize;
    items.clamp(8, 256)
}

pub(super) fn is_jpeg(path: &Path) -> bool {
    let Some(ext) = path.extension() else {
        return false;
    };
    let ext = ext.to_string_lossy().to_lowercase();
    matches!(ext.as_str(), "jpg" | "jpeg")
}

pub(super) fn div_ceil(a: u32, b: u32) -> u32 {
    if b == 0 {
        return 0;
    }
    a.div_ceil(b)
}

pub(super) fn decode_cache_item_limit() -> usize {
    let val = std::env::var("CANVAS_DECODE_CACHE_ITEMS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok());
    match val {
        Some(v) => v.clamp(1, 256),
        None => auto_decode_cache_items(decode_cache_byte_limit()),
    }
}

pub(super) fn decode_cache_byte_limit() -> u64 {
    let val = std::env::var("CANVAS_DECODE_CACHE_MB")
        .ok()
        .and_then(|v| v.parse::<u64>().ok());
    let mb = match val {
        Some(v) if v > 0 => v,
        _ => auto_decode_cache_mb().unwrap_or(512),
    };
    mb.saturating_mul(MIB)
}

pub(super) fn lod_cache_item_limit() -> usize {
    let val = std::env::var("CANVAS_LOD_CACHE_ITEMS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok());
    match val {
        Some(v) => v.clamp(1, 512),
        None => 64,
    }
}

pub(super) fn lod_cache_byte_limit() -> u64 {
    let val = std::env::var("CANVAS_LOD_CACHE_MB")
        .ok()
        .and_then(|v| v.parse::<u64>().ok());
    let mb = match val {
        Some(v) if v > 0 => v,
        _ => auto_lod_cache_mb().unwrap_or(256),
    };
    mb.saturating_mul(MIB)
}

fn hash_path_fnv(path: &Path) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in path.to_string_lossy().as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub(super) fn asset_key_or_hash(asset_key: u64, path: &Path) -> u64 {
    if asset_key != 0 {
        return asset_key;
    }
    hash_path_fnv(path) & 0x7FFF_FFFF_FFFF_FFFF
}

pub(super) fn file_meta(path: &Path) -> Option<(u64, u64)> {
    let meta = fs::metadata(path).ok()?;
    let len = meta.len();
    let modified_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    Some((len, modified_ms))
}

pub(super) fn allow_runtime_decode() -> bool {
    *ALLOW_RUNTIME_DECODE.get_or_init(|| {
        let val = std::env::var("CANVAS_RUNTIME_DECODE")
            .unwrap_or_default()
            .trim()
            .to_lowercase();
        if val.is_empty() {
            return true;
        }
        if matches!(val.as_str(), "0" | "false" | "no" | "off") {
            return false;
        }
        if matches!(val.as_str(), "1" | "true" | "yes" | "on") {
            return true;
        }
        true
    })
}

pub fn runtime_decode_enabled() -> bool {
    allow_runtime_decode()
}

pub fn max_decode_jobs_value() -> usize {
    *MAX_DECODE_JOBS.get_or_init(|| {
        let val = std::env::var("CANVAS_MAX_DECODE_JOBS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok());
        if let Some(v) = val {
            return v.clamp(1, 64);
        }
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let half = (cpus / 2).max(1);
        half.clamp(1, 12)
    })
}

pub fn decode_cache_item_limit_value() -> usize {
    decode_cache_item_limit()
}

pub fn decode_cache_byte_limit_value() -> u64 {
    decode_cache_byte_limit()
}

pub fn lod_cache_item_limit_value() -> usize {
    lod_cache_item_limit()
}

pub fn lod_cache_byte_limit_value() -> u64 {
    lod_cache_byte_limit()
}

pub fn total_ram_gib_value() -> Option<f32> {
    total_ram_bytes().map(|b| b as f32 / GIB as f32)
}
