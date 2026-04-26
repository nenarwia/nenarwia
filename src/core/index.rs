use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::index_sqlite::SqliteIndex;
use crate::core::loader::disk_cache;

mod json;

use json::JsonIndex;

const INDEX_VERSION: u32 = 2;
const INDEX_FILE: &str = "index_v2.json";
const INDEX_DB_FILE: &str = "index_v2.db";

enum IndexBackend {
    Sqlite(SqliteIndex),
    Json(JsonIndex),
}

#[derive(Clone, Copy, Debug)]
pub struct CachedPathMetadata {
    pub size: u64,
    pub modified_ms: u64,
    pub width: u32,
    pub height: u32,
    pub asset_key: u64,
}

pub struct MediaIndex {
    backend: IndexBackend,
}

impl MediaIndex {
    pub fn load_or_create(root: &Path) -> Self {
        match SqliteIndex::open(root) {
            Ok(db) => Self {
                backend: IndexBackend::Sqlite(db),
            },
            Err(err) => {
                log::warn!("SQLite index unavailable: {err:?}. Falling back to JSON index.");
                let json = JsonIndex::load_or_create(root);
                Self {
                    backend: IndexBackend::Json(json),
                }
            }
        }
    }

    pub fn refresh(&mut self, root: &Path) -> Vec<crate::core::scanner::FileItem> {
        match &mut self.backend {
            IndexBackend::Sqlite(db) => db.refresh(root),
            IndexBackend::Json(json) => json.refresh(root),
        }
    }

    pub fn cached_metadata_for_key(&self, key: &str) -> Option<CachedPathMetadata> {
        match &self.backend {
            IndexBackend::Sqlite(db) => db.cached_metadata_for_key(key),
            IndexBackend::Json(json) => json.cached_metadata_for_key(key),
        }
    }

    pub fn cache_metadata_for_key(&mut self, key: &str, metadata: CachedPathMetadata) {
        match &mut self.backend {
            IndexBackend::Sqlite(db) => {
                if let Err(err) = db.cache_metadata_for_key(key, metadata) {
                    log::warn!("Index cache update failed (sqlite): {err:?}");
                }
            }
            IndexBackend::Json(json) => {
                json.cache_metadata_for_key(key, metadata);
            }
        }
    }
}

fn index_path() -> PathBuf {
    disk_cache::cache_root().join(INDEX_FILE)
}

pub(crate) fn index_db_path() -> PathBuf {
    disk_cache::cache_root().join(INDEX_DB_FILE)
}

pub(crate) fn rel_path(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => path.to_string_lossy().to_string(),
    }
}

pub(crate) fn stable_path_key(path: &Path) -> String {
    let raw = path.to_string_lossy();
    #[cfg(windows)]
    {
        raw.replace('\\', "/").to_lowercase()
    }
    #[cfg(not(windows))]
    {
        raw.into_owned()
    }
}

pub(crate) fn modified_to_ms(time: io::Result<SystemTime>) -> u64 {
    let Ok(t) = time else {
        return 0;
    };
    match t.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_millis() as u64,
        Err(_) => 0,
    }
}

pub(crate) fn asset_key_for(rel_path: &str, size: u64, modified_ms: u64) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for &b in rel_path.as_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    for &b in size
        .to_le_bytes()
        .iter()
        .chain(modified_ms.to_le_bytes().iter())
    {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    hash & 0x7FFF_FFFF_FFFF_FFFF
}
