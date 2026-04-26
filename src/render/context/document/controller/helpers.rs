use std::collections::HashSet;
use std::path::PathBuf;

use crate::core::index::stable_path_key;
use crate::render::context::document::{CanvasDocumentMode, CanvasSlotPath};

pub(super) fn cleared_document_mode_for_canvas_reset(
    document_mode: &CanvasDocumentMode,
) -> CanvasDocumentMode {
    match document_mode {
        CanvasDocumentMode::Empty | CanvasDocumentMode::ImportedScene { .. } => {
            CanvasDocumentMode::empty()
        }
    }
}

pub(super) fn saved_media_paths_for_persistence(
    slot_paths: &[CanvasSlotPath],
    fallback: &[PathBuf],
) -> Vec<PathBuf> {
    let live_paths = collect_live_media_paths(slot_paths);
    if live_paths.is_empty() {
        fallback.to_vec()
    } else {
        live_paths
    }
}

fn collect_live_media_paths(slot_paths: &[CanvasSlotPath]) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for slot_path in slot_paths {
        let Some(path) = slot_path.live_path() else {
            continue;
        };
        let path = path.to_path_buf();
        if seen.insert(stable_path_key(&path)) {
            out.push(path);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{cleared_document_mode_for_canvas_reset, saved_media_paths_for_persistence};
    use crate::render::context::document::{CanvasDocumentMode, CanvasSlotPath};
    use std::path::PathBuf;

    #[test]
    fn clear_canvas_reset_drops_imported_mode_to_empty() {
        let cleared = cleared_document_mode_for_canvas_reset(&CanvasDocumentMode::imported(None));

        assert!(matches!(cleared, CanvasDocumentMode::Empty));
    }

    #[test]
    fn saved_media_paths_for_persistence_uses_fallback_when_no_live_paths_exist() {
        let fallback = vec![
            PathBuf::from("fallback_a.png"),
            PathBuf::from("fallback_b.png"),
        ];
        let slot_paths = vec![
            CanvasSlotPath::tombstone(PathBuf::from("deleted_a.png")),
            CanvasSlotPath::tombstone(PathBuf::from("deleted_b.png")),
        ];

        let persisted = saved_media_paths_for_persistence(&slot_paths, &fallback);

        assert_eq!(persisted, fallback);
    }

    #[test]
    fn saved_media_paths_for_persistence_deduplicates_live_paths() {
        let slot_paths = vec![
            CanvasSlotPath::live(PathBuf::from("dup.png")),
            CanvasSlotPath::tombstone(PathBuf::from("deleted.png")),
            CanvasSlotPath::live(PathBuf::from("dup.png")),
            CanvasSlotPath::live(PathBuf::from("other.png")),
        ];

        let persisted =
            saved_media_paths_for_persistence(&slot_paths, &[PathBuf::from("fallback.png")]);

        assert_eq!(
            persisted,
            vec![PathBuf::from("dup.png"), PathBuf::from("other.png")]
        );
    }
}
