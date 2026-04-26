use std::path::{Path, PathBuf};

pub fn derive_document_root(paths: &[PathBuf]) -> Option<PathBuf> {
    if paths.is_empty() {
        return None;
    }

    if paths.len() == 1 {
        let only = &paths[0];
        if only.is_dir() {
            return Some(only.clone());
        }
        if let Some(parent) = only.parent() {
            return Some(parent.to_path_buf());
        }
    }

    deepest_common_ancestor(paths)
}

fn deepest_common_ancestor(paths: &[PathBuf]) -> Option<PathBuf> {
    let mut common = paths.iter().find_map(|path| {
        if path.is_dir() {
            Some(path.clone())
        } else {
            path.parent().map(Path::to_path_buf)
        }
    })?;

    while !paths.iter().all(|path| path.starts_with(&common)) {
        let Some(parent) = common.parent() else {
            return None;
        };
        common = parent.to_path_buf();
    }

    Some(common)
}
