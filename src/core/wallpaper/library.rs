use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::assets::{persist_wallpaper_jpeg_bytes, persist_wallpaper_source};
use super::blur::ensure_saved_wallpaper_preview_blur;
use super::fs::{
    hash_bytes_sha256, hash_file_sha256, next_id_from_items, unix_time_ms, wallpaper_root,
};
use super::model::{SavedWallpaperEntry, WallpaperLibraryState, MAX_SAVED_WALLPAPERS};
use super::{
    WALLPAPER_INDEX_FILE, WALLPAPER_LIBRARY_VERSION, WALLPAPER_PREVIEW_BLUR_FILE,
    WALLPAPER_SOURCE_FILE,
};

pub struct WallpaperLibrary {
    root: PathBuf,
    state: WallpaperLibraryState,
}

impl WallpaperLibrary {
    pub fn load_library() -> Self {
        let root = wallpaper_root();
        Self::load_from_root(root)
    }

    pub fn load_from_root(root: PathBuf) -> Self {
        let index_path = root.join(WALLPAPER_INDEX_FILE);
        let state = match fs::read(&index_path) {
            Ok(bytes) => match serde_json::from_slice::<WallpaperLibraryState>(&bytes) {
                Ok(state) if state.version == WALLPAPER_LIBRARY_VERSION => state,
                Ok(_) => {
                    log::warn!("Wallpaper library version mismatch. Recreating wallpaper state.");
                    WallpaperLibraryState::default()
                }
                Err(err) => {
                    log::warn!("Failed to parse wallpaper library: {err:?}. Recreating.");
                    WallpaperLibraryState::default()
                }
            },
            Err(err) if err.kind() == io::ErrorKind::NotFound => WallpaperLibraryState::default(),
            Err(err) => {
                log::warn!("Failed to read wallpaper library: {err:?}. Recreating.");
                WallpaperLibraryState::default()
            }
        };

        let mut library = Self { root, state };
        library.sanitize();
        if let Err(err) = library.save_library() {
            log::warn!("Failed to save wallpaper library after sanitize: {err:?}");
        }
        library
    }

    pub fn save_library(&self) -> Result<()> {
        let index_path = self.root.join(WALLPAPER_INDEX_FILE);
        if let Some(parent) = index_path.parent() {
            fs::create_dir_all(parent).context("create wallpaper index dir")?;
        }
        let bytes =
            serde_json::to_vec_pretty(&self.state).context("serialize wallpaper library")?;
        fs::write(index_path, bytes).context("write wallpaper index")?;
        Ok(())
    }

    pub fn active_id(&self) -> Option<u64> {
        self.state.active_id
    }

    pub fn entries(&self) -> &[SavedWallpaperEntry] {
        &self.state.items
    }

    pub fn entry(&self, id: u64) -> Option<&SavedWallpaperEntry> {
        self.state.items.iter().find(|entry| entry.id == id)
    }

    pub fn create_from_new_source(
        &mut self,
        source_path: &Path,
        blur_enabled: bool,
        dim_amount: f32,
        preview_blur_max_dim: u32,
    ) -> Result<SavedWallpaperEntry> {
        let source_hash = hash_file_sha256(source_path)
            .with_context(|| format!("hash wallpaper source '{}'", source_path.display()))?;
        if let Some(existing) = self.reuse_existing_by_source_hash(
            &source_hash,
            blur_enabled,
            dim_amount,
            preview_blur_max_dim,
        )? {
            return Ok(existing);
        }

        let id = self.allocate_id();
        let stored_path = self.item_source_path(id);
        persist_wallpaper_source(source_path, &stored_path)?;
        self.sync_preview_blur_copy(id, &stored_path, blur_enabled, preview_blur_max_dim)?;

        let now = unix_time_ms();
        let entry = SavedWallpaperEntry {
            id,
            source_path: stored_path,
            source_hash,
            is_default: false,
            blur_enabled,
            dim_amount: dim_amount.clamp(0.0, 1.0),
            created_at: now,
            updated_at: now,
            last_used_at: now,
        };
        self.state.active_id = Some(id);
        self.state.items.insert(0, entry.clone());
        self.pin_inactive_default_wallpapers_to_end();
        self.trim_excess_items();
        self.save_library().context("save wallpaper library")?;
        Ok(entry)
    }

    pub fn ensure_default_wallpaper(&mut self, source_bytes: &[u8]) -> Result<SavedWallpaperEntry> {
        let source_hash = hash_bytes_sha256(source_bytes);
        if let Some(idx) = self
            .state
            .items
            .iter()
            .position(|entry| entry.source_hash == source_hash)
        {
            if self.state.items[idx].source_path.exists() {
                return self.ensure_existing_default_wallpaper_is_usable(idx);
            }

            let broken = self.state.items.remove(idx);
            if self.state.active_id == Some(broken.id) {
                self.state.active_id = None;
            }
            let item_dir = self.item_dir(broken.id);
            if let Err(err) = fs::remove_dir_all(&item_dir) {
                if err.kind() != io::ErrorKind::NotFound {
                    log::warn!(
                        "Failed to remove broken default wallpaper entry '{}': {err:?}",
                        item_dir.display()
                    );
                }
            }
        }

        let make_active = self.state.active_id.is_none();
        let id = self.allocate_id();
        let stored_path = self.item_source_path(id);
        persist_wallpaper_jpeg_bytes(source_bytes, &stored_path)?;

        let now = unix_time_ms();
        let entry = SavedWallpaperEntry {
            id,
            source_path: stored_path,
            source_hash,
            is_default: true,
            blur_enabled: false,
            dim_amount: 0.0,
            created_at: now,
            updated_at: now,
            last_used_at: now,
        };
        if make_active {
            self.state.active_id = Some(id);
            self.state.items.insert(0, entry.clone());
        } else {
            self.state.items.push(entry.clone());
        }
        self.pin_inactive_default_wallpapers_to_end();
        self.trim_excess_items();
        self.save_library().context("save wallpaper library")?;
        Ok(entry)
    }

    pub fn update_existing(
        &mut self,
        id: u64,
        blur_enabled: bool,
        dim_amount: f32,
        preview_blur_max_dim: u32,
    ) -> Result<SavedWallpaperEntry> {
        let idx = self
            .state
            .items
            .iter()
            .position(|entry| entry.id == id)
            .ok_or_else(|| anyhow::anyhow!("wallpaper entry {id} not found"))?;
        let mut entry = self.state.items.remove(idx);
        if !entry.source_path.exists() {
            anyhow::bail!(
                "wallpaper source '{}' is missing",
                entry.source_path.display()
            );
        }
        let now = unix_time_ms();
        entry.blur_enabled = blur_enabled;
        entry.dim_amount = dim_amount.clamp(0.0, 1.0);
        entry.updated_at = now;
        entry.last_used_at = now;
        self.sync_preview_blur_copy(
            entry.id,
            &entry.source_path,
            blur_enabled,
            preview_blur_max_dim,
        )?;
        self.state.active_id = Some(id);
        self.state.items.insert(0, entry.clone());
        self.pin_inactive_default_wallpapers_to_end();
        self.save_library().context("save wallpaper library")?;
        Ok(entry)
    }

    pub fn load_active_wallpaper(&mut self) -> Result<Option<SavedWallpaperEntry>> {
        let Some(active_id) = self.state.active_id else {
            return Ok(None);
        };
        if let Some(entry) = self.entry(active_id).cloned() {
            if entry.source_path.exists() {
                return Ok(Some(entry));
            }
        }
        log::warn!("Active wallpaper entry is missing on disk. Clearing active wallpaper.");
        self.state.active_id = None;
        self.state.items.retain(|entry| entry.source_path.exists());
        self.state.next_id = next_id_from_items(self.state.items.as_slice());
        self.save_library().context("save wallpaper library")?;
        Ok(None)
    }

    pub fn load_entry_for_preview(&self, id: u64) -> Option<SavedWallpaperEntry> {
        self.entry(id).cloned()
    }

    pub fn clear_active(&mut self) -> Result<()> {
        self.state.active_id = None;
        self.save_library().context("save wallpaper library")
    }

    pub fn preview_blur_path(&self, id: u64) -> PathBuf {
        self.item_dir(id).join(WALLPAPER_PREVIEW_BLUR_FILE)
    }

    fn reuse_existing_by_source_hash(
        &mut self,
        source_hash: &str,
        blur_enabled: bool,
        dim_amount: f32,
        preview_blur_max_dim: u32,
    ) -> Result<Option<SavedWallpaperEntry>> {
        let Some(idx) = self
            .state
            .items
            .iter()
            .position(|entry| entry.source_hash == source_hash)
        else {
            return Ok(None);
        };

        let mut entry = self.state.items.remove(idx);
        if !entry.source_path.exists() {
            if self.state.active_id == Some(entry.id) {
                self.state.active_id = None;
            }
            let item_dir = self.item_dir(entry.id);
            if let Err(err) = fs::remove_dir_all(&item_dir) {
                if err.kind() != io::ErrorKind::NotFound {
                    log::warn!(
                        "Failed to remove broken wallpaper entry '{}': {err:?}",
                        item_dir.display()
                    );
                }
            }
            return Ok(None);
        }

        let now = unix_time_ms();
        entry.source_hash = source_hash.to_owned();
        entry.blur_enabled = blur_enabled;
        entry.dim_amount = dim_amount.clamp(0.0, 1.0);
        entry.updated_at = now;
        entry.last_used_at = now;
        self.sync_preview_blur_copy(
            entry.id,
            &entry.source_path,
            blur_enabled,
            preview_blur_max_dim,
        )?;
        self.state.active_id = Some(entry.id);
        self.state.items.insert(0, entry.clone());
        self.pin_inactive_default_wallpapers_to_end();
        self.save_library().context("save wallpaper library")?;
        Ok(Some(entry))
    }

    fn ensure_existing_default_wallpaper_is_usable(
        &mut self,
        idx: usize,
    ) -> Result<SavedWallpaperEntry> {
        self.state.items[idx].is_default = true;
        if self.state.active_id.is_some() {
            let entry = self.state.items[idx].clone();
            self.pin_inactive_default_wallpapers_to_end();
            self.save_library().context("save wallpaper library")?;
            return Ok(entry);
        }

        let mut entry = self.state.items.remove(idx);
        let now = unix_time_ms();
        entry.is_default = true;
        entry.updated_at = now;
        entry.last_used_at = now;
        self.state.active_id = Some(entry.id);
        self.state.items.insert(0, entry.clone());
        self.save_library().context("save wallpaper library")?;
        Ok(entry)
    }

    fn allocate_id(&mut self) -> u64 {
        let next = self.state.next_id.max(1);
        self.state.next_id = next.saturating_add(1);
        next
    }

    fn pin_inactive_default_wallpapers_to_end(&mut self) {
        let active_id = self.state.active_id;
        let mut pinned = Vec::new();
        let mut idx = 0;
        while idx < self.state.items.len() {
            let entry = &self.state.items[idx];
            if entry.is_default && Some(entry.id) != active_id {
                pinned.push(self.state.items.remove(idx));
            } else {
                idx += 1;
            }
        }
        self.state.items.extend(pinned);
    }

    fn trim_excess_items(&mut self) {
        while self.state.items.len() > MAX_SAVED_WALLPAPERS {
            let active_id = self.state.active_id;
            let remove_idx = self
                .state
                .items
                .iter()
                .rposition(|entry| !entry.is_default && Some(entry.id) != active_id)
                .or_else(|| {
                    self.state
                        .items
                        .iter()
                        .rposition(|entry| Some(entry.id) != active_id)
                })
                .unwrap_or_else(|| self.state.items.len().saturating_sub(1));
            let removed = self.state.items.remove(remove_idx);
            if self.state.active_id == Some(removed.id) {
                self.state.active_id = None;
            }
            let item_dir = self.item_dir(removed.id);
            if let Err(err) = fs::remove_dir_all(&item_dir) {
                if err.kind() != io::ErrorKind::NotFound {
                    log::warn!(
                        "Failed to remove old wallpaper entry '{}': {err:?}",
                        item_dir.display()
                    );
                }
            }
        }
    }

    fn sanitize(&mut self) {
        self.state.version = WALLPAPER_LIBRARY_VERSION;
        self.state.items.retain(|entry| entry.source_path.exists());
        self.state
            .items
            .sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at));
        self.pin_inactive_default_wallpapers_to_end();
        self.trim_excess_items();
        if !self
            .state
            .items
            .iter()
            .any(|entry| Some(entry.id) == self.state.active_id)
        {
            self.state.active_id = None;
        }
        self.state.next_id = next_id_from_items(self.state.items.as_slice());
    }

    fn item_dir(&self, id: u64) -> PathBuf {
        self.root.join("items").join(format!("{id:016x}"))
    }

    fn item_source_path(&self, id: u64) -> PathBuf {
        self.item_dir(id).join(WALLPAPER_SOURCE_FILE)
    }

    fn sync_preview_blur_copy(
        &self,
        id: u64,
        source_path: &Path,
        blur_enabled: bool,
        preview_blur_max_dim: u32,
    ) -> Result<()> {
        let blur_path = self.preview_blur_path(id);
        if blur_enabled {
            ensure_saved_wallpaper_preview_blur(source_path, &blur_path, preview_blur_max_dim)?;
        } else if let Err(err) = fs::remove_file(&blur_path) {
            if err.kind() != io::ErrorKind::NotFound {
                return Err(err).with_context(|| {
                    format!("remove wallpaper preview blur '{}'", blur_path.display())
                });
            }
        }
        Ok(())
    }
}
