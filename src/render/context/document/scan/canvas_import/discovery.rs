use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::core::index::{asset_key_for, modified_to_ms, stable_path_key, MediaIndex};

#[derive(Clone)]
pub(super) struct ImportCandidate {
    pub(super) path: PathBuf,
    pub(super) known_dimensions: Option<(u32, u32)>,
}

pub(super) fn cached_dimensions_for_unchanged_path(
    index: &MediaIndex,
    path: &Path,
) -> Option<(u32, u32)> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() {
        return None;
    }
    let modified_ms = modified_to_ms(meta.modified());
    let cache_key = stable_path_key(path);
    let asset_key = asset_key_for(&cache_key, meta.len(), modified_ms);
    let cached = index.cached_metadata_for_key(&cache_key)?;
    let unchanged = cached.size == meta.len()
        && cached.modified_ms == modified_ms
        && cached.asset_key == asset_key
        && cached.width > 0
        && cached.height > 0;
    unchanged.then_some((cached.width, cached.height))
}

pub(super) fn for_each_image_paths(
    paths: &[PathBuf],
    mut on_path: impl FnMut(&Path) -> bool,
) -> usize {
    let mut found = 0usize;

    for path in paths {
        if path.is_file() {
            found = found.saturating_add(1);
            if !on_path(path) {
                break;
            }
            continue;
        }
        if !path.is_dir() {
            continue;
        }

        let mut keep_scanning = true;
        for entry in WalkDir::new(path)
            .sort_by_file_name()
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            found = found.saturating_add(1);
            if !on_path(entry.path()) {
                keep_scanning = false;
                break;
            }
        }

        if !keep_scanning {
            break;
        }
    }

    found
}
