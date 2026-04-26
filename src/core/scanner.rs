use std::path::{Path, PathBuf};

use crate::core::{color, formats};
use walkdir::WalkDir;

#[derive(Clone, Debug)]
pub struct FileItem {
    pub id: u64,
    pub asset_key: u64,
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
}

pub struct Scanner;

impl Scanner {
    pub fn scan_image_paths(root: &Path) -> Vec<PathBuf> {
        let mut results = Vec::new();
        Self::for_each_image_path(root, |path| {
            results.push(path.to_owned());
            true
        });
        results
    }

    pub fn for_each_image_path(root: &Path, mut on_path: impl FnMut(&Path) -> bool) -> usize {
        let mut found = 0usize;
        log::info!("Scanning directory: {}", root.to_string_lossy());

        let walker = WalkDir::new(root).sort_by_file_name().into_iter();
        for entry in walker.filter_entry(|e| !is_hidden(e)) {
            let Ok(entry) = entry else {
                continue;
            };
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            if formats::is_wic_only_path(path) {
                color::probe_wic_codec_once(path);
            }
            if !formats::is_supported_path(path) {
                continue;
            }

            found = found.saturating_add(1);
            if !on_path(path) {
                break;
            }
        }

        log::info!("Found {} valid media files.", found);
        found
    }
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

// Support list is centralized in core::formats.
