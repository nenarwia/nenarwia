use crate::core::loader::AsyncLoader;
use crate::render::{context::document::CanvasSlotPath, scene::Scene};

pub(super) struct BootstrapInitState {
    pub scene: Scene,
    pub slot_paths: Vec<CanvasSlotPath>,
    pub loader: AsyncLoader,
}

pub(super) fn build_bootstrap_state() -> BootstrapInitState {
    log::info!("Startup with empty canvas.");
    let (scene, slot_paths) = Scene::from_files(Vec::new());
    let loader = AsyncLoader::start();

    BootstrapInitState {
        scene,
        slot_paths: slot_paths.into_iter().map(CanvasSlotPath::live).collect(),
        loader,
    }
}
