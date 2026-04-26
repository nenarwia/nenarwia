use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::core::index::{stable_path_key, MediaIndex};
use crate::core::scanner::FileItem;
use crate::render::context::document::CanvasSlotPath;
use crate::render::context::state::RenderContext;
use crate::render::scene::Scene;

use super::file_item::build_canvas_file_item_from_known_metadata;
use super::import::append_unique_paths;

impl RenderContext {
    pub fn refresh_canvas_media(&mut self) -> Result<bool, String> {
        let refresh_paths = tombstone_refresh_paths(&self.slot_paths);
        if refresh_paths.is_empty() {
            return Ok(false);
        }

        self.enqueue_tombstone_refresh_paths(refresh_paths.as_slice());
        self.kickoff_pending_canvas_scan_if_idle()?;
        Ok(true)
    }

    pub(crate) fn fill_empty_canvas_slot_from_path(
        &mut self,
        slot_id: u64,
        path: &Path,
    ) -> Result<(), String> {
        let idx = self
            .scene
            .index_for_id(slot_id)
            .ok_or_else(|| format!("Slot {slot_id} no longer exists."))?;
        let slot_item_id = self
            .scene
            .index_to_id
            .get(idx)
            .copied()
            .ok_or_else(|| format!("Slot {slot_id} is out of bounds."))?;

        if manual_slot_fill_conflicts_with_live_path(&self.slot_paths, idx, path) {
            return Err(format!(
                "Image '{}' is already present on the canvas.",
                path.display()
            ));
        }

        let mut metadata_index = self
            .active_document_asset_root_cloned()
            .as_deref()
            .map(MediaIndex::load_or_create);
        let file = build_canvas_file_item_from_known_metadata(
            metadata_index.as_mut(),
            path,
            slot_item_id,
            None,
        )
        .ok_or_else(|| format!("Failed to open image '{}'.", path.display()))?;

        let document = &mut self.document;
        apply_manual_slot_fill(&mut document.scene, &mut document.slot_paths, idx, file)?;
        append_unique_paths(&mut self.media_paths, [path.to_path_buf()]);
        self.document_revision = self.document_revision.wrapping_add(1).max(1);
        self.apply_scene_append_effect();
        Ok(())
    }
}

fn tombstone_refresh_paths(slot_paths: &[CanvasSlotPath]) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut refresh_paths = Vec::new();
    for slot_path in slot_paths {
        if !slot_path.is_tombstone() {
            continue;
        }
        let remembered = slot_path.remembered_path().to_path_buf();
        if seen.insert(remembered.clone()) {
            refresh_paths.push(remembered);
        }
    }

    refresh_paths
}

fn manual_slot_fill_conflicts_with_live_path(
    slot_paths: &[CanvasSlotPath],
    target_idx: usize,
    path: &Path,
) -> bool {
    let target_key = stable_path_key(path);
    slot_paths.iter().enumerate().any(|(idx, slot_path)| {
        idx != target_idx
            && slot_path
                .live_path()
                .map(|live| stable_path_key(live) == target_key)
                .unwrap_or(false)
    })
}

fn apply_manual_slot_fill(
    scene: &mut Scene,
    slot_paths: &mut [CanvasSlotPath],
    idx: usize,
    file: FileItem,
) -> Result<(), String> {
    let Some(slot_path) = slot_paths.get(idx) else {
        return Err(format!("Slot index {idx} is out of bounds."));
    };
    if !slot_path.is_tombstone() {
        return Err(format!("Slot index {idx} is not empty."));
    }
    scene
        .restore_item_media_slot(idx, file.asset_key, (file.width, file.height))
        .ok_or_else(|| format!("Failed to restore media into slot index {idx}."))?;
    slot_paths[idx] = CanvasSlotPath::live(file.path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        apply_manual_slot_fill, manual_slot_fill_conflicts_with_live_path, tombstone_refresh_paths,
    };
    use crate::core::scanner::FileItem;
    use crate::render::context::document::CanvasSlotPath;
    use crate::render::scene::Scene;
    use std::path::PathBuf;

    #[test]
    fn tombstone_refresh_paths_only_include_tombstone_files() {
        let refresh_paths = tombstone_refresh_paths(&[
            CanvasSlotPath::live(PathBuf::from("live_a.png")),
            CanvasSlotPath::tombstone(PathBuf::from("deleted_a.png")),
            CanvasSlotPath::live(PathBuf::from("live_b.png")),
            CanvasSlotPath::tombstone(PathBuf::from("deleted_b.png")),
        ]);

        assert_eq!(
            refresh_paths,
            vec![
                PathBuf::from("deleted_a.png"),
                PathBuf::from("deleted_b.png")
            ]
        );
    }

    #[test]
    fn tombstone_refresh_paths_deduplicate_same_tombstone_path() {
        let refresh_paths = tombstone_refresh_paths(&[
            CanvasSlotPath::tombstone(PathBuf::from("deleted.png")),
            CanvasSlotPath::tombstone(PathBuf::from("deleted.png")),
            CanvasSlotPath::live(PathBuf::from("live.png")),
        ]);

        assert_eq!(refresh_paths, vec![PathBuf::from("deleted.png")]);
    }

    #[test]
    fn manual_slot_fill_replaces_tombstone_path() {
        let (mut scene, paths) = Scene::from_files(vec![make_file(1), make_file(2)]);
        let mut slot_paths: Vec<_> = paths.into_iter().map(CanvasSlotPath::live).collect();
        scene.clear_item_media_slot(1);
        slot_paths[1] = CanvasSlotPath::tombstone(PathBuf::from("deleted.png"));

        apply_manual_slot_fill(
            &mut scene,
            &mut slot_paths,
            1,
            FileItem {
                id: 2,
                asset_key: 999,
                path: PathBuf::from("manual_fill.png"),
                width: 800,
                height: 600,
            },
        )
        .expect("manual slot fill");

        assert_eq!(
            slot_paths[1],
            CanvasSlotPath::live(PathBuf::from("manual_fill.png"))
        );
        assert_eq!(scene.asset_keys[1], 999);
        assert_eq!(scene.item_dimensions[1], (800, 600));
    }

    #[test]
    fn manual_slot_fill_detects_duplicate_live_path() {
        let slot_paths = vec![
            CanvasSlotPath::live(PathBuf::from("a.png")),
            CanvasSlotPath::tombstone(PathBuf::from("deleted.png")),
        ];

        assert!(manual_slot_fill_conflicts_with_live_path(
            &slot_paths,
            1,
            PathBuf::from("a.png").as_path()
        ));
        assert!(!manual_slot_fill_conflicts_with_live_path(
            &slot_paths,
            1,
            PathBuf::from("b.png").as_path()
        ));
    }

    fn make_file(id: u64) -> FileItem {
        FileItem {
            id,
            asset_key: id + 100,
            path: PathBuf::from(format!("actions_file_{id}.png")),
            width: 640,
            height: 360,
        }
    }
}
