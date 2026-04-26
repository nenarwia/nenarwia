use crate::render::context::RenderContext;

pub(super) fn reveal_slot_in_explorer(slot_id: u64, ctx: &mut RenderContext) {
    match ctx.slot_path_for_explorer_reveal(slot_id) {
        Ok(path) => {
            if let Err(err) = std::thread::Builder::new()
                .name("reveal-in-explorer".to_string())
                .spawn(move || {
                    if let Err(err) = crate::core::file_manager::reveal_path(&path) {
                        log::warn!("Failed to reveal slot source in Explorer: {}", err);
                    }
                })
            {
                log::warn!("Failed to start Explorer reveal thread: {}", err);
            }
        }
        Err(err) => {
            log::warn!("Failed to reveal slot source in Explorer: {}", err);
        }
    }

    ctx.canvas_context_menu.close();
    ctx.mark_redraw_pending();
}

pub(super) fn open_cache_folder_in_explorer() {
    let path = crate::core::loader::disk_cache::cache_root();
    if let Err(err) = std::fs::create_dir_all(&path) {
        log::warn!(
            "Failed to prepare cache folder '{}': {}",
            path.display(),
            err
        );
        return;
    }

    if let Err(err) = std::thread::Builder::new()
        .name("open-cache-folder".to_string())
        .spawn(move || {
            if let Err(err) = crate::core::file_manager::reveal_path(&path) {
                log::warn!("Failed to open cache folder in Explorer: {}", err);
            }
        })
    {
        log::warn!("Failed to start cache folder thread: {}", err);
    }
}
