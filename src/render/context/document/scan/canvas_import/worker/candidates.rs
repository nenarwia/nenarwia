use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::core::index::{stable_path_key, MediaIndex};
use crate::core::scanner::FileItem;
use crate::render::context::document::actions::build_canvas_file_item_from_known_metadata;
use crate::render::context::document::scan::canvas_import::discovery::ImportCandidate;
use crate::render::context::document::RestoredSlot;

use super::super::discovery::{cached_dimensions_for_unchanged_path, for_each_image_paths};
use super::super::TombstoneRestoreSeed;

pub(super) struct CandidateBuckets {
    pub(super) cached_restore: Vec<RestoreMatchCandidate>,
    pub(super) uncached_restore: Vec<RestoreMatchCandidate>,
    pub(super) cached_import: Vec<ImportCandidate>,
    pub(super) uncached_import: Vec<ImportCandidate>,
}

#[derive(Clone, Debug)]
pub(super) struct RestoreMatchCandidate {
    pub(super) path: PathBuf,
    pub(super) known_dimensions: Option<(u32, u32)>,
    pub(super) slots: Vec<TombstoneRestoreSeed>,
}

pub(super) fn build_tombstone_lookup(
    tombstone_slots: Vec<TombstoneRestoreSeed>,
) -> HashMap<String, Vec<TombstoneRestoreSeed>> {
    let mut lookup = HashMap::new();
    for slot in tombstone_slots {
        lookup
            .entry(stable_path_key(slot.path.as_path()))
            .or_insert_with(Vec::new)
            .push(slot);
    }
    lookup
}

pub(super) fn collect_candidate_buckets(
    paths: &[PathBuf],
    seen_paths: &mut HashSet<String>,
    tombstone_lookup: &mut HashMap<String, Vec<TombstoneRestoreSeed>>,
    metadata_index: Option<&MediaIndex>,
) -> CandidateBuckets {
    let mut buckets = CandidateBuckets {
        cached_restore: Vec::new(),
        uncached_restore: Vec::new(),
        cached_import: Vec::new(),
        uncached_import: Vec::new(),
    };

    let _ = for_each_image_paths(paths, |path| {
        if !crate::core::formats::is_supported_path(path) {
            return true;
        }
        let path_key = stable_path_key(path);
        if !seen_paths.insert(path_key.clone()) {
            return true;
        }

        let known_dimensions =
            metadata_index.and_then(|index| cached_dimensions_for_unchanged_path(index, path));
        if let Some(slots) = tombstone_lookup.remove(&path_key) {
            let candidate = RestoreMatchCandidate {
                path: path.to_path_buf(),
                known_dimensions,
                slots,
            };
            if candidate.known_dimensions.is_some() {
                buckets.cached_restore.push(candidate);
            } else {
                buckets.uncached_restore.push(candidate);
            }
        } else {
            let candidate = ImportCandidate {
                path: path.to_path_buf(),
                known_dimensions,
            };
            if candidate.known_dimensions.is_some() {
                buckets.cached_import.push(candidate);
            } else {
                buckets.uncached_import.push(candidate);
            }
        }
        true
    });

    buckets
}

pub(super) fn materialize_restore_candidate(
    metadata_index: Option<&mut MediaIndex>,
    candidate: RestoreMatchCandidate,
) -> Option<Vec<RestoredSlot>> {
    let seed = candidate.slots.first()?;
    let file = build_canvas_file_item_from_known_metadata(
        metadata_index,
        candidate.path.as_path(),
        seed.id,
        candidate.known_dimensions,
    )?;

    Some(
        candidate
            .slots
            .into_iter()
            .map(|slot| {
                let mut restored_file = file.clone();
                restored_file.id = slot.id;
                RestoredSlot {
                    idx: slot.idx,
                    file: restored_file,
                }
            })
            .collect(),
    )
}

pub(super) fn materialize_import_candidate(
    metadata_index: Option<&mut MediaIndex>,
    candidate: ImportCandidate,
    next_id: u64,
) -> Option<FileItem> {
    build_canvas_file_item_from_known_metadata(
        metadata_index,
        candidate.path.as_path(),
        next_id,
        candidate.known_dimensions,
    )
}

#[cfg(test)]
mod tests {
    use super::{build_tombstone_lookup, collect_candidate_buckets};
    use std::collections::HashSet;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::core::index::{
        asset_key_for, modified_to_ms, stable_path_key, CachedPathMetadata, MediaIndex,
    };
    use crate::render::context::document::scan::canvas_import::TombstoneRestoreSeed;

    #[test]
    fn tombstone_lookup_groups_slots_by_path() {
        let lookup = build_tombstone_lookup(vec![
            TombstoneRestoreSeed {
                idx: 1,
                id: 10,
                path: PathBuf::from("a.png"),
            },
            TombstoneRestoreSeed {
                idx: 2,
                id: 11,
                path: PathBuf::from("a.png"),
            },
        ]);

        assert_eq!(lookup.len(), 1);
        assert_eq!(lookup.values().next().unwrap().len(), 2);
    }

    #[test]
    fn duplicate_seen_paths_are_skipped() {
        let root = std::env::temp_dir().join("canvas_import_worker_dupe_collection");
        let _ = std::fs::create_dir_all(&root);
        let path = root.join("dup.png");
        let _ = std::fs::write(&path, b"x");

        let mut seen = HashSet::from([crate::core::index::stable_path_key(path.as_path())]);
        let mut tombstones = build_tombstone_lookup(Vec::new());
        let buckets = collect_candidate_buckets(&[path], &mut seen, &mut tombstones, None);

        assert!(buckets.cached_restore.is_empty());
        assert!(buckets.uncached_restore.is_empty());
        assert!(buckets.cached_import.is_empty());
        assert!(buckets.uncached_import.is_empty());
    }

    #[test]
    fn collect_candidate_buckets_separates_cached_restore_and_uncached_import() {
        let root = unique_test_dir("worker_candidate_buckets");
        fs::create_dir_all(&root).expect("create root");
        let restored = root.join("restored.png");
        let new_media = root.join("new_media.png");
        fs::write(&restored, b"x").expect("write restored");
        fs::write(&new_media, b"y").expect("write new media");

        let mut index = MediaIndex::load_or_create(root.as_path());
        cache_dimensions(&mut index, restored.as_path(), 640, 360);

        let mut seen = HashSet::new();
        let mut tombstones = build_tombstone_lookup(vec![TombstoneRestoreSeed {
            idx: 3,
            id: 77,
            path: restored.clone(),
        }]);
        let buckets = collect_candidate_buckets(
            &[restored.clone(), new_media.clone()],
            &mut seen,
            &mut tombstones,
            Some(&index),
        );

        assert_eq!(buckets.cached_restore.len(), 1);
        assert!(buckets.uncached_restore.is_empty());
        assert!(buckets.cached_import.is_empty());
        assert_eq!(buckets.uncached_import.len(), 1);
        assert_eq!(buckets.cached_restore[0].path, restored);
        assert_eq!(buckets.uncached_import[0].path, new_media);

        let _ = fs::remove_dir_all(root);
    }

    fn cache_dimensions(index: &mut MediaIndex, path: &std::path::Path, width: u32, height: u32) {
        let meta = fs::metadata(path).expect("metadata");
        let modified_ms = modified_to_ms(meta.modified());
        let cache_key = stable_path_key(path);
        index.cache_metadata_for_key(
            &cache_key,
            CachedPathMetadata {
                size: meta.len(),
                modified_ms,
                width,
                height,
                asset_key: asset_key_for(&cache_key, meta.len(), modified_ms),
            },
        );
    }

    fn unique_test_dir(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("nenarwia_{label}_{suffix}"))
    }
}
