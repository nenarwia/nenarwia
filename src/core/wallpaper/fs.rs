use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use crate::core::loader::disk_cache;

use super::model::SavedWallpaperEntry;
use super::WALLPAPER_ROOT_DIR;

pub fn wallpaper_root() -> PathBuf {
    let base = disk_cache::state_root();
    base.join(WALLPAPER_ROOT_DIR)
}

pub fn unix_time_ms() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis() as u64,
        Err(_) => 0,
    }
}

pub fn next_id_from_items(items: &[SavedWallpaperEntry]) -> u64 {
    items
        .iter()
        .map(|entry| entry.id)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
        .max(1)
}

pub fn hash_file_sha256(path: &Path) -> Result<String> {
    let file = fs::File::open(path).with_context(|| format!("open '{}'", path.display()))?;
    let mut reader = io::BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];

    loop {
        let read = reader
            .read(&mut buf)
            .with_context(|| format!("read '{}'", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

pub fn hash_bytes_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
