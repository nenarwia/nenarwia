use std::path::Path;

use crate::core::index::{
    asset_key_for, modified_to_ms, stable_path_key, CachedPathMetadata, MediaIndex,
};
use crate::core::scanner::FileItem;

pub(crate) fn build_canvas_file_item_from_known_metadata(
    metadata_index: Option<&mut MediaIndex>,
    path: &Path,
    id: u64,
    known_dimensions: Option<(u32, u32)>,
) -> Option<FileItem> {
    if !crate::core::formats::is_supported_path(path) {
        return None;
    }

    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() {
        return None;
    }

    let modified_ms = modified_to_ms(meta.modified());
    let cache_key = stable_path_key(path);
    let asset_key = asset_key_for(&cache_key, meta.len(), modified_ms);

    let (width, height) = resolve_canvas_dimensions_for_path(
        metadata_index,
        &cache_key,
        path,
        meta.len(),
        modified_ms,
        asset_key,
        known_dimensions,
    );

    Some(FileItem {
        id,
        asset_key,
        path: path.to_path_buf(),
        width,
        height,
    })
}

fn resolve_canvas_dimensions_for_path(
    metadata_index: Option<&mut MediaIndex>,
    cache_key: &str,
    path: &Path,
    size: u64,
    modified_ms: u64,
    asset_key: u64,
    known_dimensions: Option<(u32, u32)>,
) -> (u32, u32) {
    let known_dimensions = known_dimensions.filter(|(width, height)| *width > 0 && *height > 0);

    if let Some(index) = metadata_index {
        if let Some((width, height)) = known_dimensions {
            index.cache_metadata_for_key(
                cache_key,
                CachedPathMetadata {
                    size,
                    modified_ms,
                    width,
                    height,
                    asset_key,
                },
            );
            return (width, height);
        }

        if let Some(cached) = index.cached_metadata_for_key(cache_key) {
            let unchanged = cached.size == size
                && cached.modified_ms == modified_ms
                && cached.asset_key == asset_key
                && cached.width > 0
                && cached.height > 0;
            if unchanged {
                return (cached.width, cached.height);
            }
        }

        if let Some((width, height)) = probe_canvas_dimensions(path) {
            index.cache_metadata_for_key(
                cache_key,
                CachedPathMetadata {
                    size,
                    modified_ms,
                    width,
                    height,
                    asset_key,
                },
            );
            return (width, height);
        }
    } else if let Some((width, height)) = known_dimensions.or_else(|| probe_canvas_dimensions(path))
    {
        return (width, height);
    }

    (1, 1)
}

fn probe_canvas_dimensions(path: &Path) -> Option<(u32, u32)> {
    crate::core::color::image_dimensions_any(path).or_else(|| {
        crate::core::color::decode_rgba8_srgb_thumbnail(path, 1024)
            .ok()
            .map(|decoded| (decoded.width, decoded.height))
    })
}

#[cfg(test)]
mod tests {
    use super::build_canvas_file_item_from_known_metadata;
    use crate::core::index::{stable_path_key, MediaIndex};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn build_canvas_file_item_uses_known_dimensions_and_caches_metadata() {
        let root = unique_test_dir("file_item_known_dimensions");
        std::fs::create_dir_all(&root).expect("create root");
        let path = root.join("known.png");
        std::fs::write(&path, b"not-a-real-png").expect("write image");

        let mut index = MediaIndex::load_or_create(root.as_path());
        let item = build_canvas_file_item_from_known_metadata(
            Some(&mut index),
            path.as_path(),
            77,
            Some((640, 360)),
        )
        .expect("build file item");

        assert_eq!(item.id, 77);
        assert_eq!(item.path, path);
        assert_eq!((item.width, item.height), (640, 360));

        let cache_key = stable_path_key(item.path.as_path());
        let cached = index
            .cached_metadata_for_key(&cache_key)
            .expect("cached metadata");
        assert_eq!((cached.width, cached.height), (640, 360));
        assert_eq!(cached.asset_key, item.asset_key);

        let _ = std::fs::remove_dir_all(root);
    }

    fn unique_test_dir(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("nenarwia_{label}_{suffix}"))
    }
}
