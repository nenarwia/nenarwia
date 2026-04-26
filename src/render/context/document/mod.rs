mod actions;
mod active;
mod controller;
mod lifecycle;
mod model;
mod reveal_in_explorer;
pub mod scan;
mod trash_delete;

#[cfg(test)]
pub use active::SystemFileDropState;
pub use active::{ActiveDocumentState, DocumentBackgroundState};
pub use model::{
    derive_document_root, CanvasDocumentMode, CanvasSlotPath, CanvasTabState, RestoredSlot,
    ScanResult,
};
