use std::collections::HashSet;
use std::path::PathBuf;

use crate::core::index::stable_path_key;
use crate::render::context::document::{derive_document_root, CanvasDocumentMode};
use crate::render::context::state::RenderContext;

impl RenderContext {
    pub fn add_media_to_canvas_from_paths(&mut self, paths: &[PathBuf]) -> Result<(), String> {
        if paths.is_empty() {
            return Ok(());
        }

        append_unique_paths(&mut self.media_paths, paths.iter().cloned());
        let asset_root_hint = canvas_import_asset_root_hint(&self.document_mode, paths);
        self.prepare_active_document_for_canvas_import(asset_root_hint);
        self.enqueue_canvas_import_paths(paths);
        let result = self.kickoff_pending_canvas_scan_if_idle();
        if result.is_ok() {
            self.mark_redraw_pending();
        }
        result.map(|_| ())
    }

    fn prepare_active_document_for_canvas_import(&mut self, asset_root_hint: Option<PathBuf>) {
        match &mut self.document_mode {
            CanvasDocumentMode::ImportedScene {
                asset_root_hint: current_root_hint,
            } => {
                if current_root_hint.is_none() {
                    *current_root_hint = asset_root_hint;
                }
            }
            _ => self.promote_active_document_to_imported(asset_root_hint),
        }
    }
}

fn canvas_import_asset_root_hint(
    document_mode: &CanvasDocumentMode,
    paths: &[PathBuf],
) -> Option<PathBuf> {
    let import_root = derive_document_root(paths);

    match document_mode {
        CanvasDocumentMode::ImportedScene {
            asset_root_hint, ..
        } => asset_root_hint.clone().or(import_root),
        CanvasDocumentMode::Empty => import_root,
    }
}

pub(super) fn append_unique_paths(
    pending: &mut Vec<PathBuf>,
    incoming: impl IntoIterator<Item = PathBuf>,
) {
    let mut seen = pending
        .iter()
        .map(|path| stable_path_key(path))
        .collect::<HashSet<_>>();
    for path in incoming {
        if seen.insert(stable_path_key(&path)) {
            pending.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::canvas_import_asset_root_hint;
    use crate::render::context::document::CanvasDocumentMode;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn empty_document_canvas_import_uses_import_root() {
        let root = unique_test_dir("empty_document_root");
        let dropped = root.join("nested").join("album");
        std::fs::create_dir_all(&dropped).expect("create dropped folder");

        let document_mode = CanvasDocumentMode::empty();
        let asset_root_hint = canvas_import_asset_root_hint(&document_mode, &[dropped.clone()]);

        assert_eq!(asset_root_hint, Some(dropped));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn imported_scene_canvas_import_keeps_existing_root() {
        let root = unique_test_dir("imported_scene_root");
        let existing = root.join("existing");
        let dropped = root.join("nested").join("album");
        std::fs::create_dir_all(&dropped).expect("create dropped folder");

        let document_mode = CanvasDocumentMode::imported(Some(existing.clone()));
        let asset_root_hint = canvas_import_asset_root_hint(&document_mode, &[dropped]);

        assert_eq!(asset_root_hint, Some(existing));

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
