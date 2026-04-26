use std::path::PathBuf;
use std::sync::OnceLock;

static CACHE_ROOT_PATH: OnceLock<PathBuf> = OnceLock::new();
static LIBRARY_ROOT_PATH: OnceLock<PathBuf> = OnceLock::new();
static STATE_ROOT_PATH: OnceLock<PathBuf> = OnceLock::new();

fn base_cache_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(dir).join("CanvasEngine");
    }
    std::env::temp_dir().join("CanvasEngine")
}

pub fn cache_root() -> PathBuf {
    CACHE_ROOT_PATH
        .get_or_init(|| base_cache_dir().join(".canvas_cache").join("v1"))
        .clone()
}

pub fn library_root() -> PathBuf {
    LIBRARY_ROOT_PATH
        .get_or_init(|| base_cache_dir().join(".canvas_library").join("v1"))
        .clone()
}

pub fn state_root() -> PathBuf {
    STATE_ROOT_PATH
        .get_or_init(|| base_cache_dir().join(".canvas_state").join("v1"))
        .clone()
}

fn lod_root(asset_key: u64) -> PathBuf {
    cache_root().join("lod").join(format!("{:016x}", asset_key))
}

pub(super) fn lod_root_dir(asset_key: u64) -> PathBuf {
    lod_root(asset_key)
}
