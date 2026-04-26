use std::fs;
use std::io;

use crate::core::pack::PackKind;

use super::paths::{cache_root, library_root};
use super::shared;

const DEFAULT_RUNTIME_BUDGET_BYTES: u64 = 10 * 1024 * 1024 * 1024; // 10GiB

pub fn bump_library_generation() {
    shared::bump_library_generation();
}

pub fn bump_runtime_generation() {
    shared::bump_runtime_generation();
}

fn runtime_budget_bytes() -> u64 {
    if let Ok(val) = std::env::var("CANVAS_DISK_BUDGET_BYTES") {
        if let Ok(parsed) = val.parse::<u64>() {
            return parsed;
        }
    }
    if let Ok(val) = std::env::var("CANVAS_DISK_BUDGET_GB") {
        if let Ok(parsed) = val.parse::<u64>() {
            return parsed.saturating_mul(1024 * 1024 * 1024);
        }
    }
    DEFAULT_RUNTIME_BUDGET_BYTES
}

pub(super) fn enforce_runtime_budget() -> io::Result<()> {
    let budget = runtime_budget_bytes();
    shared::with_runtime_writer(|pack| {
        let bytes = pack.bytes_for_kind(PackKind::Runtime)?;
        if bytes <= budget {
            return Ok(());
        }
        let to_free = bytes.saturating_sub(budget);
        let _ = pack.evict_lru(PackKind::Runtime, to_free)?;
        Ok(())
    })
}

pub fn clear_runtime_cache() -> io::Result<()> {
    let root = cache_root();
    if root.exists() {
        shared::close_runtime_handles();
        fs::remove_dir_all(root)?;
    }
    bump_runtime_generation();
    Ok(())
}

pub fn delete_runtime_asset(asset_key: u64) -> io::Result<()> {
    shared::with_runtime_writer(|pack| pack.delete_asset_pages(asset_key))?;
    let lod_root = super::paths::lod_root_dir(asset_key);
    if lod_root.exists() {
        let _ = fs::remove_dir_all(lod_root);
    }
    bump_runtime_generation();
    Ok(())
}

pub fn compact_runtime_pack() -> io::Result<()> {
    shared::close_runtime_handles();
    crate::core::pack::compact_pack(&cache_root())?;
    bump_runtime_generation();
    Ok(())
}

pub fn compact_library_pack() -> io::Result<()> {
    shared::close_library_handles();
    crate::core::pack::compact_pack(&library_root())?;
    bump_library_generation();
    Ok(())
}
