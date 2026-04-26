use std::sync::atomic::{AtomicU64, Ordering};

mod assets;
mod open;
mod pages;

const DEFAULT_CHUNK_SIZE: u64 = 16 * 1024 * 1024;
const MIN_CHUNK_SIZE: u64 = 8 * 1024 * 1024;
const MAX_CHUNK_SIZE: u64 = 64 * 1024 * 1024;

static PAGE_EPOCH: AtomicU64 = AtomicU64::new(1);

fn next_epoch() -> u64 {
    PAGE_EPOCH.fetch_add(1, Ordering::Relaxed)
}
