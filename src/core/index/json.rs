use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::core::color;
use crate::core::scanner::{FileItem, Scanner};

use super::{
    asset_key_for, index_path, modified_to_ms, rel_path, CachedPathMetadata, INDEX_VERSION,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct IndexEntry {
    id: u64,
    #[serde(default)]
    asset_key: u64,
    rel_path: String,
    size: u64,
    modified_ms: u64,
    width: u32,
    height: u32,
    present: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct IndexFile {
    version: u32,
    root: String,
    next_id: u64,
    entries: Vec<IndexEntry>,
}

pub(super) struct JsonIndex {
    file: IndexFile,
    path_to_idx: HashMap<String, usize>,
}

impl JsonIndex {
    pub fn load_or_create(root: &Path) -> Self {
        let path = index_path();
        let file = match fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<IndexFile>(&bytes) {
                Ok(mut f) => {
                    if f.version != INDEX_VERSION {
                        log::warn!("Index version mismatch, rebuilding index.");
                        f = IndexFile::new(root);
                    }
                    f
                }
                Err(e) => {
                    log::warn!("Failed to parse index: {e:?}. Rebuilding index.");
                    IndexFile::new(root)
                }
            },
            Err(_) => IndexFile::new(root),
        };

        let mut idx = Self {
            file,
            path_to_idx: HashMap::new(),
        };
        idx.rebuild_map();
        idx
    }

    pub fn refresh(&mut self, root: &Path) -> Vec<FileItem> {
        let paths = Scanner::scan_image_paths(root);
        let mut seen: HashSet<String> = HashSet::with_capacity(paths.len());

        for path in paths {
            let rel = rel_path(root, &path);
            seen.insert(rel.clone());

            let meta = match fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let size = meta.len();
            let modified_ms = modified_to_ms(meta.modified());

            if let Some(&idx) = self.path_to_idx.get(&rel) {
                let entry = &mut self.file.entries[idx];
                entry.present = true;

                let changed = entry.size != size
                    || entry.modified_ms != modified_ms
                    || entry.width == 0
                    || entry.height == 0
                    || entry.asset_key == 0;

                if changed {
                    let (w, h) = color::image_dimensions_any(&path).unwrap_or((0, 0));
                    entry.size = size;
                    entry.modified_ms = modified_ms;
                    entry.width = w;
                    entry.height = h;
                    entry.asset_key = asset_key_for(&rel, size, modified_ms);
                }
            } else {
                let (w, h) = color::image_dimensions_any(&path).unwrap_or((0, 0));
                let id = self.file.next_id;
                self.file.next_id = self.file.next_id.saturating_add(1);

                let entry = IndexEntry {
                    id,
                    asset_key: asset_key_for(&rel, size, modified_ms),
                    rel_path: rel.clone(),
                    size,
                    modified_ms,
                    width: w,
                    height: h,
                    present: true,
                };

                self.file.entries.push(entry);
                let new_idx = self.file.entries.len() - 1;
                self.path_to_idx.insert(rel, new_idx);
            }
        }

        for entry in self.file.entries.iter_mut() {
            if !seen.contains(&entry.rel_path) {
                entry.present = false;
            }
        }

        self.file.root = root.to_string_lossy().to_string();
        let _ = self.save();

        self.build_items(root)
    }

    pub fn save(&self) -> io::Result<()> {
        let path = index_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(&self.file)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{e:?}")))?;
        fs::write(path, bytes)
    }

    pub fn build_items(&self, root: &Path) -> Vec<FileItem> {
        let mut out = Vec::with_capacity(self.file.entries.len());

        for entry in self.file.entries.iter() {
            if !entry.present {
                continue;
            }

            let path = root.join(&entry.rel_path);

            out.push(FileItem {
                id: entry.id,
                asset_key: entry.asset_key,
                path,
                width: entry.width,
                height: entry.height,
            });
        }

        out
    }

    pub fn cached_metadata_for_key(&self, key: &str) -> Option<CachedPathMetadata> {
        let idx = *self.path_to_idx.get(key)?;
        let entry = self.file.entries.get(idx)?;
        Some(CachedPathMetadata {
            size: entry.size,
            modified_ms: entry.modified_ms,
            width: entry.width,
            height: entry.height,
            asset_key: entry.asset_key,
        })
    }

    pub fn cache_metadata_for_key(&mut self, key: &str, metadata: CachedPathMetadata) {
        let rel = key.to_string();
        if let Some(&idx) = self.path_to_idx.get(&rel) {
            let entry = &mut self.file.entries[idx];
            entry.size = metadata.size;
            entry.modified_ms = metadata.modified_ms;
            entry.width = metadata.width;
            entry.height = metadata.height;
            entry.asset_key = metadata.asset_key;
        } else {
            let id = self.file.next_id;
            self.file.next_id = self.file.next_id.saturating_add(1);
            self.file.entries.push(IndexEntry {
                id,
                asset_key: metadata.asset_key,
                rel_path: rel.clone(),
                size: metadata.size,
                modified_ms: metadata.modified_ms,
                width: metadata.width,
                height: metadata.height,
                present: false,
            });
            let new_idx = self.file.entries.len() - 1;
            self.path_to_idx.insert(rel, new_idx);
        }

        let _ = self.save();
    }

    fn rebuild_map(&mut self) {
        self.path_to_idx.clear();
        for (idx, entry) in self.file.entries.iter().enumerate() {
            self.path_to_idx.insert(entry.rel_path.clone(), idx);
        }
    }
}

impl IndexFile {
    fn new(root: &Path) -> Self {
        Self {
            version: INDEX_VERSION,
            root: root.to_string_lossy().to_string(),
            next_id: 0,
            entries: Vec::new(),
        }
    }
}
