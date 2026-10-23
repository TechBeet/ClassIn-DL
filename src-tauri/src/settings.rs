//! Settings module - persists user preferences to a JSON file.
use crate::models::AppSettings;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};
use tauri::{AppHandle, Manager};

static SETTINGS_PATH: OnceLock<RwLock<Option<PathBuf>>> = OnceLock::new();
static INITIALIZED: OnceLock<std::sync::atomic::AtomicBool> = OnceLock::new();

fn slot() -> &'static RwLock<Option<PathBuf>> {
    SETTINGS_PATH.get_or_init(|| RwLock::new(None))
}

/// Initialise the settings file location from the app's config directory.
pub fn init(app: &AppHandle) {
    let flag = INITIALIZED.get_or_init(|| std::sync::atomic::AtomicBool::new(false));
    if flag.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    if let Ok(dir) = app.path().config_dir() {
        let target = dir.join("classin-downloader").join("settings.json");
        if let Ok(mut w) = slot().write() {
            *w = Some(target);
        }
    }
}

fn path() -> PathBuf {
    if let Ok(r) = slot().read() {
        if let Some(p) = r.as_ref() {
            return p.clone();
        }
    }
    let fallback = std::env::temp_dir().join("classin-downloader").join("settings.json");
    if let Ok(mut w) = slot().write() {
        if w.is_none() {
            *w = Some(fallback.clone());
        }
    }
    fallback
}

/// Load persisted settings, falling back to defaults.
pub fn load_settings() -> AppSettings {
    let p = path();
    if let Ok(data) = std::fs::read_to_string(&p) {
        if let Ok(s) = serde_json::from_str::<AppSettings>(&data) {
            return s;
        }
    }
    AppSettings::default()
}

/// Persist the current settings to disk.
pub fn save_settings(settings: &AppSettings) -> Result<()> {
    let p = path();
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(settings)?;
    std::fs::write(&p, json)?;
    log::info!("Settings saved to {}", p.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_roundtrip() {
        let mut s = AppSettings::default();
        s.theme = "dark".to_string();
        let json = serde_json::to_string(&s).unwrap();
        let parsed: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.theme, "dark");
    }
}