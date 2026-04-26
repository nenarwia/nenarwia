use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock, RwLock};

use anyhow::{Context, Result};

use crate::core::loader::disk_cache::state_root;

use super::types::AppSettings;

const APP_SETTINGS_FILE: &str = "app_settings.json";

static APP_SETTINGS_CACHE: OnceLock<RwLock<AppSettings>> = OnceLock::new();
static APP_SETTINGS_SAVE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn load_app_settings() -> AppSettings {
    app_settings_cache()
        .read()
        .expect("app settings cache lock")
        .clone()
}

pub(super) fn update_app_settings(update: impl FnOnce(&mut AppSettings)) -> Result<()> {
    let _save_guard = app_settings_save_lock()
        .lock()
        .expect("app settings save lock");
    let mut settings = load_app_settings();
    update(&mut settings);
    save_app_settings_locked(&settings)
}

fn app_settings_cache() -> &'static RwLock<AppSettings> {
    APP_SETTINGS_CACHE.get_or_init(|| RwLock::new(load_app_settings_from_disk()))
}

fn app_settings_save_lock() -> &'static Mutex<()> {
    APP_SETTINGS_SAVE_LOCK.get_or_init(|| Mutex::new(()))
}

fn load_app_settings_from_disk() -> AppSettings {
    let path = app_settings_path();
    match fs::read(&path) {
        Ok(bytes) => match serde_json::from_slice::<AppSettings>(strip_utf8_bom(&bytes)) {
            Ok(settings) => settings.sanitized(),
            Err(err) => {
                log::warn!("Failed to parse app settings '{}': {err:?}", path.display());
                AppSettings::default()
            }
        },
        Err(err) if err.kind() == io::ErrorKind::NotFound => AppSettings::default(),
        Err(err) => {
            log::warn!("Failed to read app settings '{}': {err:?}", path.display());
            AppSettings::default()
        }
    }
}

fn save_app_settings_locked(settings: &AppSettings) -> Result<()> {
    let settings = settings.clone().sanitized();
    write_app_settings_to_disk(&settings)?;
    replace_cached_app_settings(settings);
    Ok(())
}

fn write_app_settings_to_disk(settings: &AppSettings) -> Result<()> {
    let path = app_settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("create app settings dir")?;
    }
    let bytes = serde_json::to_vec_pretty(settings).context("serialize app settings")?;
    fs::write(&path, bytes).with_context(|| format!("write app settings '{}'", path.display()))?;
    Ok(())
}

fn replace_cached_app_settings(settings: AppSettings) {
    let initial = settings.clone();
    let cache = APP_SETTINGS_CACHE.get_or_init(|| RwLock::new(initial));
    *cache.write().expect("app settings cache lock") = settings;
}

fn app_settings_path() -> PathBuf {
    state_root().join(APP_SETTINGS_FILE)
}

fn strip_utf8_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes)
}

#[cfg(test)]
mod tests {
    use super::strip_utf8_bom;

    #[test]
    fn strip_utf8_bom_removes_utf8_bom_only_when_present() {
        assert_eq!(strip_utf8_bom(&[0xEF, 0xBB, 0xBF, b'{']), &[b'{']);
        assert_eq!(strip_utf8_bom(b"{\"version\":1}"), b"{\"version\":1}");
    }
}
