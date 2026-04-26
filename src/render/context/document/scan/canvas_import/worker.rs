use std::path::{Path, PathBuf};
use std::sync::mpsc;

use crate::core::scanner::FileItem;
use crate::render::context::document::ScanResult;
use crate::render::layout::SceneLayoutCursor;

use super::tail_refill::{TailAppendSeed, TailRefillSeed};
use super::TombstoneRestoreSeed;

mod candidates;
mod chunks;
mod emission;

#[cfg(test)]
mod tests;

use chunks::CanvasScanMergeState;

const INITIAL_LOOKAHEAD_BLOCKS: usize = 1;
const STEADY_LOOKAHEAD_BLOCKS: usize = 2;

pub(in crate::render::context::document::scan) fn stream_canvas_scan_merge(
    paths: Vec<PathBuf>,
    existing_files: Vec<FileItem>,
    tombstone_slots: Vec<TombstoneRestoreSeed>,
    next_id: u64,
    asset_root_hint: Option<&Path>,
    epoch: u64,
    tab_id: u64,
    document_revision: u64,
    start_cursor: SceneLayoutCursor,
    start_index: usize,
    start_block_id: u64,
    start_bounds: Option<[f32; 4]>,
    tail_refill_seed: Option<TailRefillSeed>,
    tail_append_seed: Option<TailAppendSeed>,
    allow_new_imports: bool,
    tx: &mpsc::Sender<ScanResult>,
) {
    let mut state = CanvasScanMergeState::new(
        next_id,
        asset_root_hint,
        existing_files,
        tombstone_slots,
        start_cursor,
        start_index,
        start_block_id,
        start_bounds,
        tail_refill_seed,
        tail_append_seed,
        allow_new_imports,
        epoch,
        tab_id,
        document_revision,
        tx,
    );

    let buckets = state.collect_candidates(paths.as_slice());
    state.materialize_restore_candidates(
        buckets
            .cached_restore
            .into_iter()
            .chain(buckets.uncached_restore.into_iter()),
    );
    state.materialize_import_candidates(
        buckets
            .cached_import
            .into_iter()
            .chain(buckets.uncached_import.into_iter()),
    );
    state.finish();
}
