mod apply;
mod canvas_import;

use std::path::PathBuf;
use std::sync::mpsc;

use crate::render::context::document::ScanResult;
use crate::render::context::state::RenderContext;

use self::canvas_import::{
    snapshot_scene_file_items, snapshot_scene_tail_append_seed, snapshot_scene_tail_refill_seed,
    snapshot_scene_tombstone_slots, stream_canvas_scan_merge,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CanvasScanKind {
    Import,
    TombstoneRefresh,
}

impl CanvasScanKind {
    fn allows_new_imports(self) -> bool {
        matches!(self, Self::Import)
    }

    fn thread_name(self) -> &'static str {
        match self {
            Self::Import => "media-scan-canvas-import",
            Self::TombstoneRefresh => "media-scan-canvas-refresh",
        }
    }
}

impl RenderContext {
    pub(crate) fn kickoff_canvas_import_scan(&mut self, paths: &[PathBuf]) -> Result<(), String> {
        self.kickoff_canvas_scan(paths, CanvasScanKind::Import)
    }

    pub(crate) fn kickoff_tombstone_refresh_scan(
        &mut self,
        paths: &[PathBuf],
    ) -> Result<(), String> {
        self.kickoff_canvas_scan(paths, CanvasScanKind::TombstoneRefresh)
    }

    fn kickoff_canvas_scan(
        &mut self,
        paths: &[PathBuf],
        scan_kind: CanvasScanKind,
    ) -> Result<(), String> {
        if paths.is_empty() {
            return Ok(());
        }
        if self.background.scan_inflight {
            return Err("Canvas scan is already in progress.".to_string());
        }

        self.background.scan_epoch = self.background.scan_epoch.wrapping_add(1);
        let epoch = self.background.scan_epoch;
        let tab_id = self.active_tab_id();
        let document_revision = self.document_revision;
        let existing_files = snapshot_scene_file_items(self);
        let tombstone_slots = snapshot_scene_tombstone_slots(self);
        let next_file_id = self
            .scene
            .index_to_id
            .iter()
            .copied()
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let asset_root_hint = self.active_document_asset_root_cloned();
        let start_cursor = self.scene.layout_cursor;
        let start_index = self.scene.total_count;
        let start_block_id = self.scene.next_block_id();
        let start_bounds = self.scene.layout_bounds();
        let tail_refill_seed = if scan_kind.allows_new_imports() {
            snapshot_scene_tail_refill_seed(self)
        } else {
            None
        };
        let tail_append_seed = if scan_kind.allows_new_imports() && tail_refill_seed.is_none() {
            snapshot_scene_tail_append_seed(self)
        } else {
            None
        };
        let (tx, rx) = mpsc::channel::<ScanResult>();
        let paths = paths.to_vec();
        self.background.scan_rx = Some(rx);
        self.background.scan_inflight = true;

        std::thread::Builder::new()
            .name(scan_kind.thread_name().to_string())
            .spawn(move || {
                stream_canvas_scan_merge(
                    paths,
                    existing_files,
                    tombstone_slots,
                    next_file_id,
                    asset_root_hint.as_deref(),
                    epoch,
                    tab_id,
                    document_revision,
                    start_cursor,
                    start_index,
                    start_block_id,
                    start_bounds,
                    tail_refill_seed,
                    tail_append_seed,
                    scan_kind.allows_new_imports(),
                    &tx,
                );
            })
            .map(|_| ())
            .map_err(|err| format!("Failed to start canvas scan worker: {err}"))
    }

    pub(crate) fn enqueue_canvas_import_paths(&mut self, paths: &[PathBuf]) {
        append_unique_paths(
            &mut self.background.pending_canvas_import_paths,
            paths.iter().cloned(),
        );
        append_unique_paths(
            &mut self.background.canvas_import_resume_paths,
            paths.iter().cloned(),
        );
    }

    pub(crate) fn enqueue_tombstone_refresh_paths(&mut self, paths: &[PathBuf]) {
        append_unique_paths(
            &mut self.background.pending_tombstone_refresh_paths,
            paths.iter().cloned(),
        );
    }

    pub(crate) fn kickoff_pending_canvas_scan_if_idle(&mut self) -> Result<bool, String> {
        if self.background.scan_inflight {
            return Ok(false);
        }

        let (scan_kind, paths) = if !self.background.pending_canvas_import_paths.is_empty() {
            (
                CanvasScanKind::Import,
                std::mem::take(&mut self.background.pending_canvas_import_paths),
            )
        } else if !self.background.pending_tombstone_refresh_paths.is_empty() {
            (
                CanvasScanKind::TombstoneRefresh,
                std::mem::take(&mut self.background.pending_tombstone_refresh_paths),
            )
        } else {
            return Ok(false);
        };

        let kickoff = match scan_kind {
            CanvasScanKind::Import => self.kickoff_canvas_import_scan(paths.as_slice()),
            CanvasScanKind::TombstoneRefresh => {
                self.kickoff_tombstone_refresh_scan(paths.as_slice())
            }
        };
        match kickoff {
            Ok(()) => Ok(true),
            Err(err) => {
                match scan_kind {
                    CanvasScanKind::Import => self.background.pending_canvas_import_paths = paths,
                    CanvasScanKind::TombstoneRefresh => {
                        self.background.pending_tombstone_refresh_paths = paths
                    }
                }
                Err(err)
            }
        }
    }

    pub(crate) fn resume_canvas_import_scan_after_tab_activation(&mut self) {
        if self.background.canvas_import_resume_paths.is_empty() {
            return;
        }

        let resume_paths = self.background.canvas_import_resume_paths.clone();
        append_unique_paths(
            &mut self.background.pending_canvas_import_paths,
            resume_paths,
        );

        if let Err(err) = self.kickoff_pending_canvas_scan_if_idle() {
            log::warn!(
                "Failed to resume canvas import worker after tab activation: {}",
                err
            );
        }
    }

    pub(crate) fn poll_document_scan_results(&mut self) {
        let Some(rx) = self.background.scan_rx.take() else {
            return;
        };

        const MAX_SCAN_RESULTS_PER_FRAME: usize = 1;
        let mut finished = false;
        let mut processed = 0usize;
        loop {
            if processed >= MAX_SCAN_RESULTS_PER_FRAME {
                break;
            }
            match rx.try_recv() {
                Ok(result) => {
                    processed = processed.saturating_add(1);
                    let final_scan = result.final_scan;
                    if result.epoch == self.background.scan_epoch
                        && result.tab_id == self.active_tab_id()
                        && result.document_revision == self.document_revision
                    {
                        self.apply_scan_result(result);
                    }
                    if final_scan {
                        finished = true;
                        break;
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    finished = true;
                    break;
                }
            }
        }

        if finished {
            self.background.scan_inflight = false;
            let queued_started = match self.kickoff_pending_canvas_scan_if_idle() {
                Ok(started) => started,
                Err(err) => {
                    log::warn!("Failed to start queued canvas scan worker: {}", err);
                    false
                }
            };
            if !queued_started
                && self.background.pending_canvas_import_paths.is_empty()
                && self.background.pending_tombstone_refresh_paths.is_empty()
            {
                self.background.canvas_import_resume_paths.clear();
            }
        } else {
            self.background.scan_rx = Some(rx);
        }
    }
}

fn append_unique_paths(pending: &mut Vec<PathBuf>, incoming: impl IntoIterator<Item = PathBuf>) {
    let mut seen = pending
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    for path in incoming {
        if seen.insert(path.clone()) {
            pending.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::append_unique_paths;
    use std::path::PathBuf;

    #[test]
    fn append_unique_paths_preserves_first_seen_order() {
        let mut pending = vec![PathBuf::from("a"), PathBuf::from("b")];

        append_unique_paths(
            &mut pending,
            vec![
                PathBuf::from("b"),
                PathBuf::from("c"),
                PathBuf::from("a"),
                PathBuf::from("d"),
            ],
        );

        assert_eq!(
            pending,
            vec![
                PathBuf::from("a"),
                PathBuf::from("b"),
                PathBuf::from("c"),
                PathBuf::from("d"),
            ]
        );
    }

    #[test]
    fn append_unique_paths_handles_empty_pending_queue() {
        let mut pending = Vec::new();

        append_unique_paths(
            &mut pending,
            vec![
                PathBuf::from("folder"),
                PathBuf::from("folder"),
                PathBuf::from("image.png"),
            ],
        );

        assert_eq!(
            pending,
            vec![PathBuf::from("folder"), PathBuf::from("image.png")]
        );
    }
}
