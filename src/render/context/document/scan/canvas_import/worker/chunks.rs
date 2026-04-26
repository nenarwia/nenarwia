use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use crate::core::index::{stable_path_key, MediaIndex};
use crate::core::scanner::FileItem;
use crate::render::context::document::scan::canvas_import::discovery::ImportCandidate;
use crate::render::context::document::{RestoredSlot, ScanResult};
use crate::render::layout::{BlockLayoutPlanner, SceneLayoutCursor, BLOCK_FILE_CAP};

use super::super::tail_refill::{
    fixed_grid_cursor_for_append, merge_bounds_opt, TailAppendSeed, TailAppendState,
    TailRefillSeed, TailRefillState,
};
use super::super::TombstoneRestoreSeed;
use super::candidates::{
    build_tombstone_lookup, collect_candidate_buckets, materialize_import_candidate,
    materialize_restore_candidate, CandidateBuckets, RestoreMatchCandidate,
};
use super::emission::{empty_batch_for_cursor, send_append_snapshot, send_planned_append};
use super::{INITIAL_LOOKAHEAD_BLOCKS, STEADY_LOOKAHEAD_BLOCKS};

pub(super) struct CanvasScanMergeState<'a> {
    seen_paths: HashSet<String>,
    tombstone_lookup: std::collections::HashMap<String, Vec<TombstoneRestoreSeed>>,
    metadata_index: Option<MediaIndex>,
    pending_restores: Vec<RestoredSlot>,
    pending_chunk_files: Vec<FileItem>,
    tail_refill: Option<TailRefillState>,
    tail_append: Option<TailAppendState>,
    planner: Option<BlockLayoutPlanner>,
    chunk_blocks: usize,
    observed_new_count: usize,
    observed_restore_count: usize,
    next_id: u64,
    allow_new_imports: bool,
    start_cursor: SceneLayoutCursor,
    start_index: usize,
    start_block_id: u64,
    start_bounds: Option<[f32; 4]>,
    epoch: u64,
    tab_id: u64,
    document_revision: u64,
    tx: &'a mpsc::Sender<ScanResult>,
}

impl<'a> CanvasScanMergeState<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        next_id: u64,
        asset_root_hint: Option<&Path>,
        existing_files: Vec<FileItem>,
        tombstone_slots: Vec<TombstoneRestoreSeed>,
        start_cursor: SceneLayoutCursor,
        start_index: usize,
        start_block_id: u64,
        start_bounds: Option<[f32; 4]>,
        tail_refill_seed: Option<TailRefillSeed>,
        tail_append_seed: Option<TailAppendSeed>,
        allow_new_imports: bool,
        epoch: u64,
        tab_id: u64,
        document_revision: u64,
        tx: &'a mpsc::Sender<ScanResult>,
    ) -> Self {
        let seen_paths = existing_files
            .iter()
            .map(|file| stable_path_key(file.path.as_path()))
            .collect();
        let planner = if tail_refill_seed.is_none() && tail_append_seed.is_none() {
            Some(BlockLayoutPlanner::new_with_seed_bounds(
                start_cursor,
                start_block_id,
                start_bounds,
            ))
        } else {
            None
        };

        Self {
            seen_paths,
            tombstone_lookup: build_tombstone_lookup(tombstone_slots),
            metadata_index: asset_root_hint.map(MediaIndex::load_or_create),
            pending_restores: Vec::new(),
            pending_chunk_files: Vec::new(),
            tail_refill: tail_refill_seed.map(TailRefillState::new),
            tail_append: tail_append_seed.map(TailAppendState::new),
            planner,
            chunk_blocks: INITIAL_LOOKAHEAD_BLOCKS,
            observed_new_count: 0,
            observed_restore_count: 0,
            next_id,
            allow_new_imports,
            start_cursor,
            start_index,
            start_block_id,
            start_bounds,
            epoch,
            tab_id,
            document_revision,
            tx,
        }
    }

    pub(super) fn collect_candidates(&mut self, paths: &[PathBuf]) -> CandidateBuckets {
        collect_candidate_buckets(
            paths,
            &mut self.seen_paths,
            &mut self.tombstone_lookup,
            self.metadata_index.as_ref(),
        )
    }

    pub(super) fn materialize_restore_candidates(
        &mut self,
        candidates: impl IntoIterator<Item = RestoreMatchCandidate>,
    ) {
        for candidate in candidates {
            if let Some(restores) =
                materialize_restore_candidate(self.metadata_index.as_mut(), candidate)
            {
                self.observed_restore_count =
                    self.observed_restore_count.saturating_add(restores.len());
                self.pending_restores.extend(restores);
            }
        }
    }

    pub(super) fn materialize_import_candidates(
        &mut self,
        candidates: impl IntoIterator<Item = ImportCandidate>,
    ) {
        for candidate in candidates {
            if !self.allow_new_imports {
                continue;
            }
            let Some(file) =
                materialize_import_candidate(self.metadata_index.as_mut(), candidate, self.next_id)
            else {
                continue;
            };
            self.next_id = self.next_id.saturating_add(1);
            self.observed_new_count = self.observed_new_count.saturating_add(1);
            self.pending_chunk_files.push(file);

            if self.pending_chunk_files.len()
                < pending_chunk_target_len(
                    self.tail_refill.as_ref(),
                    self.tail_append.as_ref(),
                    self.chunk_blocks,
                )
            {
                continue;
            }

            if !self.flush_pending_chunk(false) {
                return;
            }
            self.chunk_blocks = STEADY_LOOKAHEAD_BLOCKS;
        }
    }

    pub(super) fn finish(&mut self) {
        if self.observed_new_count == 0 && self.observed_restore_count == 0 {
            return;
        }

        if self.pending_chunk_files.is_empty() && self.pending_restores.is_empty() {
            let final_cursor = self
                .planner
                .as_ref()
                .map(BlockLayoutPlanner::cursor)
                .unwrap_or(self.start_cursor);
            let _ = send_append_snapshot(
                Vec::new(),
                crate::render::scene::SceneAppendBlocksBatch {
                    layout_width: final_cursor.target_side,
                    layout_height: final_cursor.target_side,
                    layout_cursor: final_cursor,
                    tail: None,
                    blocks: Vec::new(),
                },
                self.epoch,
                self.tab_id,
                self.document_revision,
                self.tx,
                true,
            );
            return;
        }

        let _ = self.flush_pending_chunk(true);
    }

    fn flush_pending_chunk(&mut self, final_chunk: bool) -> bool {
        if self.pending_chunk_files.is_empty() && self.pending_restores.is_empty() {
            return true;
        }

        let base_cursor = self
            .planner
            .as_ref()
            .map(BlockLayoutPlanner::cursor)
            .unwrap_or(self.start_cursor);
        let chunk_cursor = fixed_grid_cursor_for_append(base_cursor);

        if let Some(planner) = self.planner.as_mut() {
            planner.grow_target_for_total_items(
                self.start_index.saturating_add(self.observed_new_count),
            );
        }

        let mut tail_append_plan = None;
        if let Some(tail) = self.tail_append.as_mut() {
            if tail.needs_more() && !self.pending_chunk_files.is_empty() {
                let take = tail
                    .remaining_capacity()
                    .min(self.pending_chunk_files.len());
                tail_append_plan = tail.consume_chunk(&self.pending_chunk_files[..take]);
                let consumed = tail_append_plan
                    .as_ref()
                    .map(|plan| plan.entries.len())
                    .unwrap_or(0);
                if consumed > 0 {
                    self.pending_chunk_files.drain(..consumed);
                    if !tail.needs_more() && self.planner.is_none() {
                        let append_bounds = tail.current_bounds();
                        let mut next_planner = BlockLayoutPlanner::new_with_seed_bounds(
                            chunk_cursor,
                            self.start_block_id,
                            merge_bounds_opt(self.start_bounds, append_bounds),
                        );
                        next_planner.grow_target_for_total_items(
                            self.start_index.saturating_add(self.observed_new_count),
                        );
                        self.planner = Some(next_planner);
                    }
                }
            }
        }

        for file in self.pending_chunk_files.drain(..) {
            let mut consumed_by_tail = false;
            if let Some(tail) = self.tail_refill.as_mut() {
                if tail.needs_more() {
                    tail.consume_new_file(file.clone());
                    consumed_by_tail = true;
                    if !tail.needs_more() && self.planner.is_none() {
                        let refill_bounds = tail.refill_bounds();
                        let mut next_planner = BlockLayoutPlanner::new_with_seed_bounds(
                            chunk_cursor,
                            self.start_block_id,
                            merge_bounds_opt(self.start_bounds, refill_bounds),
                        );
                        next_planner.grow_target_for_total_items(
                            self.start_index.saturating_add(self.observed_new_count),
                        );
                        self.planner = Some(next_planner);
                    }
                }
            }

            if !consumed_by_tail {
                let planner = self.planner.get_or_insert_with(|| {
                    let mut next_planner = BlockLayoutPlanner::new_with_seed_bounds(
                        chunk_cursor,
                        self.start_block_id,
                        self.start_bounds,
                    );
                    next_planner.grow_target_for_total_items(
                        self.start_index.saturating_add(self.observed_new_count),
                    );
                    next_planner
                });
                planner.push(file);
            }
        }

        if final_chunk {
            if let Some(tail) = self.tail_refill.as_mut() {
                tail.finalize_at_end();
            }
            if let Some(planner) = self.planner.as_mut() {
                planner.flush_pending_block();
            }
        }

        let planner_batch = self
            .planner
            .as_mut()
            .map(|planner| planner.take_batch())
            .unwrap_or_else(|| empty_batch_for_cursor(chunk_cursor));
        let tail_plan = self
            .tail_refill
            .as_mut()
            .and_then(|tail| tail.take_plan_once());

        send_planned_append(
            std::mem::take(&mut self.pending_restores),
            planner_batch,
            tail_plan,
            tail_append_plan,
            self.tail_refill.as_ref(),
            chunk_cursor,
            self.start_bounds,
            self.epoch,
            self.tab_id,
            self.document_revision,
            self.tx,
            final_chunk,
        )
    }
}

pub(super) fn pending_chunk_target_len(
    tail_refill: Option<&TailRefillState>,
    tail_append: Option<&TailAppendState>,
    lookahead_blocks: usize,
) -> usize {
    let tail_needed = tail_refill
        .filter(|tail| tail.needs_more())
        .map(|tail| BLOCK_FILE_CAP.saturating_sub(tail.combined_len()))
        .or_else(|| {
            tail_append
                .filter(|tail| tail.needs_more())
                .map(TailAppendState::remaining_capacity)
        })
        .unwrap_or(0);
    tail_needed.saturating_add(BLOCK_FILE_CAP.saturating_mul(lookahead_blocks.max(1)))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::core::scanner::FileItem;
    use crate::render::context::document::scan::canvas_import::tail_refill::{
        TailAppendSeed, TailAppendState, TailRefillSeed, TailRefillState,
    };
    use crate::render::layout::{BlockGridAddress, BLOCK_FILE_CAP};

    use super::pending_chunk_target_len;

    #[test]
    fn pending_chunk_target_len_uses_tail_refill_remaining_capacity_first() {
        let seed = TailRefillSeed {
            block_id: 1,
            grid: BlockGridAddress { col: 0, row: 0 },
            existing_len: BLOCK_FILE_CAP - 2,
            bounds: [0.0, 0.0, 1.0, 1.0],
            files: (0..(BLOCK_FILE_CAP - 2))
                .map(|idx| FileItem {
                    id: idx as u64,
                    asset_key: idx as u64,
                    path: PathBuf::from(format!("f{idx}.png")),
                    width: 1,
                    height: 1,
                })
                .collect(),
        };
        let refill = TailRefillState::new(seed);

        assert_eq!(
            pending_chunk_target_len(Some(&refill), None, 1),
            2 + BLOCK_FILE_CAP
        );
    }

    #[test]
    fn pending_chunk_target_len_uses_tail_append_remaining_capacity() {
        let seed = TailAppendSeed {
            block_id: 1,
            grid: BlockGridAddress { col: 0, row: 0 },
            existing_len: BLOCK_FILE_CAP - 3,
            bounds: [0.0, 0.0, 1.0, 1.0],
        };
        let append = TailAppendState::new(seed);

        assert_eq!(
            pending_chunk_target_len(None, Some(&append), 2),
            3 + BLOCK_FILE_CAP * 2
        );
    }
}
