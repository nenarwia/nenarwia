use std::path::PathBuf;

use crate::core::index::{asset_key_for, modified_to_ms};
use crate::core::scanner::FileItem;
use crate::render::context::{document::CanvasSlotPath, state::RenderContext};
use crate::render::scene::Scene;

use super::tail_refill::{TailAppendSeed, TailRefillSeed};

#[derive(Clone, Debug)]
pub(in crate::render::context::document::scan) struct TombstoneRestoreSeed {
    pub idx: usize,
    pub id: u64,
    pub path: PathBuf,
}

pub(in crate::render::context::document::scan) fn snapshot_scene_file_items(
    ctx: &RenderContext,
) -> Vec<FileItem> {
    let mut out = Vec::with_capacity(ctx.slot_paths.len());
    for idx in 0..ctx.slot_paths.len() {
        if let Some(file) = scene_file_item_at_idx(
            &ctx.scene,
            &ctx.slot_paths,
            ctx.active_document_asset_root(),
            idx,
        ) {
            out.push(file);
        }
    }
    out
}

pub(in crate::render::context::document::scan) fn snapshot_scene_tail_refill_seed(
    ctx: &RenderContext,
) -> Option<TailRefillSeed> {
    snapshot_tail_refill_seed_parts(
        &ctx.scene,
        &ctx.slot_paths,
        ctx.active_document_asset_root(),
    )
}

pub(in crate::render::context::document::scan) fn snapshot_scene_tail_append_seed(
    ctx: &RenderContext,
) -> Option<TailAppendSeed> {
    snapshot_tail_append_seed_parts(&ctx.scene)
}

pub(in crate::render::context::document::scan) fn snapshot_scene_tombstone_slots(
    ctx: &RenderContext,
) -> Vec<TombstoneRestoreSeed> {
    let mut out = Vec::new();
    for (idx, slot_path) in ctx.slot_paths.iter().enumerate() {
        let CanvasSlotPath::Tombstone(path) = slot_path else {
            continue;
        };
        let id = ctx
            .scene
            .index_to_id
            .get(idx)
            .copied()
            .unwrap_or(idx as u64);
        out.push(TombstoneRestoreSeed {
            idx,
            id,
            path: path.clone(),
        });
    }
    out
}

fn snapshot_tail_refill_seed_parts(
    scene: &Scene,
    slot_paths: &[CanvasSlotPath],
    asset_root: Option<&std::path::Path>,
) -> Option<TailRefillSeed> {
    let block = scene.layout_blocks.last()?;
    if block.index_len == 0 || block.index_len >= crate::render::layout::BLOCK_FILE_CAP {
        return None;
    }

    let start = block.index_start;
    let end = start.saturating_add(block.index_len);
    let mut files = Vec::with_capacity(block.index_len);
    for idx in start..end {
        files.push(scene_file_item_at_idx(scene, slot_paths, asset_root, idx)?);
    }

    Some(TailRefillSeed {
        block_id: block.block_id,
        grid: block.grid,
        existing_len: block.index_len,
        bounds: block.bounds,
        files,
    })
}

fn snapshot_tail_append_seed_parts(scene: &Scene) -> Option<TailAppendSeed> {
    let block = scene.layout_blocks.last()?;
    if block.index_len == 0 || block.index_len >= crate::render::layout::BLOCK_FILE_CAP {
        return None;
    }

    Some(TailAppendSeed {
        block_id: block.block_id,
        grid: block.grid,
        existing_len: block.index_len,
        bounds: block.bounds,
    })
}

fn scene_file_item_at_idx(
    scene: &Scene,
    slot_paths: &[CanvasSlotPath],
    asset_root: Option<&std::path::Path>,
    idx: usize,
) -> Option<FileItem> {
    let path = slot_paths.get(idx)?.live_path()?;
    let id = scene.index_to_id.get(idx).copied().unwrap_or(idx as u64);
    let (width, height) = scene.item_dimensions.get(idx).copied().unwrap_or((1, 1));
    let asset_key = scene.asset_keys.get(idx).copied().unwrap_or_else(|| {
        let meta = std::fs::metadata(path).ok();
        let size = meta.as_ref().map(|entry| entry.len()).unwrap_or(0);
        let modified_ms = meta
            .as_ref()
            .map(|entry| modified_to_ms(entry.modified()))
            .unwrap_or_default();
        let rel = asset_root
            .and_then(|root| path.strip_prefix(root).ok())
            .map(|rel| rel.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        asset_key_for(&rel, size, modified_ms)
    });

    Some(FileItem {
        id,
        asset_key,
        path: path.to_path_buf(),
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::{snapshot_tail_append_seed_parts, snapshot_tail_refill_seed_parts};
    use crate::core::scanner::FileItem;
    use crate::render::context::document::CanvasSlotPath;
    use crate::render::layout::BLOCK_FILE_CAP;
    use crate::render::scene::Scene;
    use std::path::PathBuf;

    fn make_files(count: usize) -> Vec<FileItem> {
        (0..count)
            .map(|idx| FileItem {
                id: idx as u64 + 1,
                asset_key: idx as u64 + 100,
                path: PathBuf::from(format!("snapshot_file_{idx}.png")),
                width: 100,
                height: 100,
            })
            .collect()
    }

    #[test]
    fn tail_refill_seed_survives_tombstones_before_last_block() {
        let files = make_files(BLOCK_FILE_CAP + 1);
        let (scene, paths) = Scene::from_files(files);
        let mut slot_paths: Vec<_> = paths.into_iter().map(CanvasSlotPath::live).collect();
        slot_paths[5] = CanvasSlotPath::tombstone(PathBuf::from("deleted.png"));

        let seed = snapshot_tail_refill_seed_parts(&scene, &slot_paths, None)
            .expect("tail refill seed should exist");

        assert_eq!(seed.block_id, 1);
        assert_eq!(seed.existing_len, 1);
        assert_eq!(seed.files.len(), 1);
    }

    #[test]
    fn tail_refill_seed_is_disabled_when_last_block_contains_tombstone() {
        let files = make_files(BLOCK_FILE_CAP + 1);
        let (scene, paths) = Scene::from_files(files);
        let mut slot_paths: Vec<_> = paths.into_iter().map(CanvasSlotPath::live).collect();
        slot_paths[BLOCK_FILE_CAP] = CanvasSlotPath::tombstone(PathBuf::from("deleted_tail.png"));

        let seed = snapshot_tail_refill_seed_parts(&scene, &slot_paths, None);

        assert!(seed.is_none());
    }

    #[test]
    fn tail_append_seed_survives_when_last_block_contains_tombstone() {
        let files = make_files(BLOCK_FILE_CAP + 1);
        let (scene, paths) = Scene::from_files(files);
        let mut slot_paths: Vec<_> = paths.into_iter().map(CanvasSlotPath::live).collect();
        slot_paths[BLOCK_FILE_CAP] = CanvasSlotPath::tombstone(PathBuf::from("deleted_tail.png"));

        let refill = snapshot_tail_refill_seed_parts(&scene, &slot_paths, None);
        let append = snapshot_tail_append_seed_parts(&scene).expect("tail append seed");

        assert!(refill.is_none());
        assert_eq!(append.block_id, 1);
        assert_eq!(append.existing_len, 1);
    }
}
