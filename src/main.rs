#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod core;
mod render;
mod spatial;

use std::path::PathBuf;

use crate::core::engine::CanvasEngine;

fn main() {
    let mut args = std::env::args().skip(1);
    if let Some(cmd) = args.next() {
        match cmd.as_str() {
            "install" => run_install(args),
            "compact" => run_compact(),
            "clear-cache" => run_clear_cache(),
            _ => {
                eprintln!(
                    "Unknown command: {cmd}. Supported commands: install, compact, clear-cache."
                );
                std::process::exit(2);
            }
        }
        return;
    }

    run_viewer();
}

fn run_install(args: impl Iterator<Item = String>) {
    init_logger();

    let mut root: Option<String> = None;
    for arg in args {
        if arg == "--full" {
            std::env::set_var("CANVAS_INSTALL_FULL", "1");
        } else if !arg.starts_with('-') && root.is_none() {
            root = Some(arg);
        }
    }

    let root = root.unwrap_or_else(|| "./assets".to_string());
    let root_path = PathBuf::from(root);
    match crate::core::install::run_install(&root_path) {
        Ok(stats) => {
            log::info!(
                "Install complete: total={} installed={} skipped={} failed={}",
                stats.total,
                stats.installed,
                stats.skipped,
                stats.failed
            );
        }
        Err(err) => {
            log::error!("Install failed: {err:?}");
        }
    }
}

fn run_compact() {
    init_logger();

    if let Err(err) = crate::core::loader::disk_cache::compact_runtime_pack() {
        log::error!("Compact runtime pack failed: {err:?}");
    }
    if let Err(err) = crate::core::loader::disk_cache::compact_library_pack() {
        log::error!("Compact library pack failed: {err:?}");
    }
}

fn run_clear_cache() {
    init_logger();

    if let Err(err) = crate::core::loader::disk_cache::clear_runtime_cache() {
        log::error!("Clear cache failed: {err:?}");
    }
}

fn run_viewer() {
    CanvasEngine::run();
}

fn init_logger() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
}
