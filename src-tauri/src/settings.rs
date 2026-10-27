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

/// Internal helper for loading from a specific path (for testing).
fn load_from(p: &std::path::Path) -> AppSettings {
    if let Ok(data) = std::fs::read_to_string(p) {
        if let Ok(s) = serde_json::from_str::<AppSettings>(&data) {
            return s;
        }
    }
    AppSettings::default()
}

/// Internal helper for saving to a specific path (for testing).
fn save_to(p: &std::path::Path, settings: &AppSettings) -> Result<()> {
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(settings)?;
    std::fs::write(p, json)?;
    Ok(())
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

    #[test]
    fn test_settings_full_roundtrip() {
        use crate::models::{ColumnWidths, SortConfig};
        let s = AppSettings {
            theme: "light".to_string(),
            column_widths: ColumnWidths {
                idx: Some(80.0),
                name: Some(300.0),
                size: Some(120.0),
                progress: Some(240.0),
                status: Some(160.0),
                action: Some(130.0),
            },
            sort: SortConfig { column: "name".to_string(), ascending: true },
            max_concurrent: 8,
            download_dir: "C:\\Downloads".to_string(),
        };
        let json = serde_json::to_string(&s).unwrap();
        let parsed: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.theme, s.theme);
        assert_eq!(parsed.column_widths.idx, s.column_widths.idx);
        assert_eq!(parsed.sort.column, s.sort.column);
    }

    #[test]
    fn test_settings_default() {
        let s = AppSettings::default();
        assert_eq!(s.theme, "");
        assert_eq!(s.max_concurrent, 0);
    }

    #[test]
    fn test_load_settings_missing_file_returns_default() {
        use crate::models::AppSettings;
        let path = std::env::temp_dir().join("classin-dl-nonexistent-settings-test.json");
        let _ = std::fs::remove_file(&path);
        let settings = load_from(&path);
        assert_eq!(settings.theme, AppSettings::default().theme);
    }

    #[test]
    fn test_load_settings_corrupted_returns_default() {
        use crate::models::AppSettings;
        let path = std::env::temp_dir().join("classin-dl-corrupted-settings-test.json");
        std::fs::write(&path, "not json {{{").unwrap();
        let settings = load_from(&path);
        assert_eq!(settings.theme, AppSettings::default().theme);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_load_from_valid_file() {
        let path = std::env::temp_dir().join("classin-dl-valid-settings-test.json");
        let s = AppSettings { theme: "dark".to_string(), ..Default::default() };
        std::fs::write(&path, serde_json::to_string(&s).unwrap()).unwrap();
        let loaded = load_from(&path);
        assert_eq!(loaded.theme, "dark");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_save_settings_creates_parent_dir() {
        let path = std::env::temp_dir()
            .join("classin-dl-settings-test-dir")
            .join("sub")
            .join("settings.json");
        let s = AppSettings { theme: "light".to_string(), ..Default::default() };
        save_to(&path, &s).unwrap();
        let loaded = load_from(&path);
        assert_eq!(loaded.theme, "light");
        let _ = std::fs::remove_dir_all(std::env::temp_dir().join("classin-dl-settings-test-dir"));
    }
}