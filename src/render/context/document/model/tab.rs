use std::path::PathBuf;

use crate::render::scene::Scene;
use crate::spatial::view::ViewState;

use super::{CanvasDocumentMode, CanvasSlotPath};

pub struct CanvasTabState {
    pub id: u64,
    pub title: String,
    pub scene: Scene,
    pub slot_paths: Vec<CanvasSlotPath>,
    pub media_paths: Vec<PathBuf>,
    pub view: ViewState,
    pub document_mode: CanvasDocumentMode,
    pub document_revision: u64,
    pub auto_frame_pending: bool,
    pub canvas_import_resume_paths: Vec<PathBuf>,
}

impl CanvasTabState {
    pub fn new_blank(id: u64, width: u32, height: u32) -> Self {
        Self::new_empty_with_mode(id, width, height, CanvasDocumentMode::empty())
    }

    pub fn new_empty_with_mode(
        id: u64,
        width: u32,
        height: u32,
        document_mode: CanvasDocumentMode,
    ) -> Self {
        let (scene, slot_paths) = Scene::from_files(Vec::new());
        Self {
            id,
            title: "Untitled".to_string(),
            scene,
            slot_paths: slot_paths.into_iter().map(CanvasSlotPath::live).collect(),
            media_paths: Vec::new(),
            view: ViewState::new(width.max(1), height.max(1)),
            document_mode,
            document_revision: 1,
            auto_frame_pending: true,
            canvas_import_resume_paths: Vec::new(),
        }
    }

    pub fn placeholder(id: u64, title: String, width: u32, height: u32) -> Self {
        let (scene, slot_paths) = Scene::from_files(Vec::new());
        Self {
            id,
            title,
            scene,
            slot_paths: slot_paths.into_iter().map(CanvasSlotPath::live).collect(),
            media_paths: Vec::new(),
            view: ViewState::new(width.max(1), height.max(1)),
            document_mode: CanvasDocumentMode::empty(),
            document_revision: 1,
            auto_frame_pending: true,
            canvas_import_resume_paths: Vec::new(),
        }
    }
}
