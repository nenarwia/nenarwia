use std::time::{Duration, Instant};

use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

use crate::render::context::document::{CanvasDocumentMode, CanvasSlotPath, ScanResult};
use crate::render::context::state::RenderContext;
use crate::render::scene::Scene;

#[derive(Default)]
pub struct SystemFileDropState {
    pub paths: Vec<PathBuf>,
    pub last_at: Option<Instant>,
    pub on_canvas: bool,
}

impl SystemFileDropState {
    pub fn queue(&mut self, path: &Path, at: Instant, on_canvas_if_first: bool) -> bool {
        self.last_at = Some(at);
        if self.paths.is_empty() {
            self.on_canvas = on_canvas_if_first;
        }
        if self.paths.iter().any(|queued| queued == path) {
            return false;
        }
        self.paths.push(path.to_path_buf());
        true
    }

    pub fn should_finalize(&self, now: Instant, debounce: Duration) -> bool {
        self.last_at
            .map(|last_at| now.duration_since(last_at) >= debounce)
            .unwrap_or(false)
    }

    pub fn take_batch(&mut self) -> (Vec<PathBuf>, bool) {
        let paths = std::mem::take(&mut self.paths);
        let on_canvas = self.on_canvas;
        self.last_at = None;
        self.on_canvas = false;
        (paths, on_canvas)
    }

    pub fn clear(&mut self) {
        self.paths.clear();
        self.last_at = None;
        self.on_canvas = false;
    }
}

#[derive(Default)]
pub struct DocumentBackgroundState {
    pub scan_rx: Option<Receiver<ScanResult>>,
    pub scan_epoch: u64,
    pub scan_inflight: bool,
    pub pending_canvas_import_paths: Vec<PathBuf>,
    pub pending_tombstone_refresh_paths: Vec<PathBuf>,
    pub canvas_import_resume_paths: Vec<PathBuf>,
    pub system_file_drop: SystemFileDropState,
    pub auto_frame_pending: bool,
}

pub struct ActiveDocumentState {
    pub scene: Scene,
    pub slot_paths: Vec<CanvasSlotPath>,
    pub media_paths: Vec<PathBuf>,
    pub document_mode: CanvasDocumentMode,
    pub document_revision: u64,
    pub background: DocumentBackgroundState,
}

impl RenderContext {
    pub(crate) fn auto_frame_scene_if_needed(&mut self) {
        if !self.background.auto_frame_pending {
            return;
        }
        if self.viewport_runtime().last_changed_frame > 0 {
            return;
        }
        if self.scene.bounds().is_none() {
            return;
        }
        self.fit_scene_immediate(1.1);
        self.background.auto_frame_pending = false;
    }
}

impl Deref for RenderContext {
    type Target = ActiveDocumentState;

    fn deref(&self) -> &Self::Target {
        &self.document
    }
}

impl DerefMut for RenderContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.document
    }
}
