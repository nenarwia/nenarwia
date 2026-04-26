use crate::core::scanner::FileItem;
use crate::render::scene::SceneAppendBlocksBatch;

pub struct RestoredSlot {
    pub idx: usize,
    pub file: FileItem,
}

pub struct ScanResult {
    pub epoch: u64,
    pub tab_id: u64,
    pub document_revision: u64,
    pub restores: Vec<RestoredSlot>,
    pub batch: SceneAppendBlocksBatch,
    pub final_scan: bool,
}
