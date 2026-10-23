// ClassIn Video Downloader - Tauri 2.x Backend
mod clipboard;
mod downloader;
mod logger;
mod m3u8;
mod models;
mod parser;
mod settings;

use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

pub struct AppState {
    pub manager: Mutex<downloader::DownloadManager>,
    pub watcher: Mutex<Option<clipboard::ClipboardWatcher>>,
}

#[tauri::command]
async fn parse_videos(content: String, max_concurrent: usize) -> Result<Vec<models::VideoItem>, String> {
    parser::parse_videos(&content, max_concurrent)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn read_clipboard() -> Result<String, String> {
    clipboard::read_clipboard().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn write_clipboard(text: String) -> Result<(), String> {
    clipboard::write_clipboard(&text).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn copy_get_lesson_record_info() -> Result<(), String> {
    clipboard::copy_get_lesson_record_info_command()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_clipboard_watch(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut guard = state.watcher.lock().await;
    if let Some(w) = guard.as_ref() {
        w.reset();
        return Ok(());
    }
    let watcher = clipboard::start_clipboard_watcher(app);
    *guard = Some(watcher);
    Ok(())
}

#[tauri::command]
async fn stop_clipboard_watch(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let guard = state.watcher.lock().await;
    if let Some(w) = guard.as_ref() {
        w.cancel();
    }
    Ok(())
}

#[tauri::command]
async fn start_downloads(
    app: tauri::AppHandle,
    videos: Vec<models::VideoItem>,
    dir: String,
    max_concurrent: usize,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mgr = state.manager.lock().await.clone();
    mgr.start_downloads(app, videos, dir, max_concurrent)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn cancel_download(
    video_id: u32,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mgr = state.manager.lock().await.clone();
    mgr.cancel_download(video_id).await;
    Ok(())
}

#[tauri::command]
async fn cancel_all_downloads(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mgr = state.manager.lock().await.clone();
    mgr.cancel_all().await;
    Ok(())
}

#[tauri::command]
async fn pause_download(video_id: u32, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mgr = state.manager.lock().await.clone();
    mgr.set_paused(video_id, true).await;
    Ok(())
}

#[tauri::command]
async fn resume_download(video_id: u32, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mgr = state.manager.lock().await.clone();
    mgr.set_paused(video_id, false).await;
    Ok(())
}

#[tauri::command]
async fn select_directory() -> Result<String, String> {
    let picked = tokio::task::spawn_blocking(|| {
        rfd::FileDialog::new().pick_folder()
    })
    .await
    .map_err(|e| e.to_string())?;
    picked
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "No directory selected".to_string())
}

#[tauri::command]
fn open_folder(path: String) -> Result<(), String> {
    downloader::DownloadManager::open_download_folder(&path);
    Ok(())
}

#[tauri::command]
fn get_default_dir() -> String {
    dirs::download_dir()
        .or_else(dirs::home_dir)
        .map(|d| d.join("ClassInDownloads").to_string_lossy().to_string())
        .unwrap_or_else(|| "ClassInDownloads".to_string())
}

#[tauri::command]
fn initialize_log() -> Result<(), String> {
    logger::initialize_log_file().map_err(|e| e.to_string())
}

#[tauri::command]
fn read_logs() -> Result<Vec<String>, String> {
    logger::read_log().map_err(|e| e.to_string())
}

#[tauri::command]
fn load_settings() -> models::AppSettings {
    settings::load_settings()
}

#[tauri::command]
fn save_settings(new_settings: models::AppSettings) -> Result<(), String> {
    settings::save_settings(&new_settings).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_version(app: tauri::AppHandle) -> Result<String, String> {
    Ok(app.package_info().version.to_string())
}


#[tauri::command]
fn show_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
    Ok(())
}
fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            settings::init(app.handle());

            let concurrency = {
                let s = settings::load_settings();
                if s.max_concurrent == 0 { 5 } else { s.max_concurrent }
            };
            let manager = downloader::DownloadManager::new(concurrency, num_cpus::get() * 4);
            app.manage(AppState {
                manager: Mutex::new(manager),
                watcher: Mutex::new(None),
            });

            let _ = logger::initialize_log_file();

            let handle = app.handle().clone();
            let watcher = clipboard::start_clipboard_watcher(handle.clone());
            tauri::async_runtime::block_on(async {
                let state = handle.state::<AppState>();
                *state.watcher.lock().await = Some(watcher);
            });

            let _ = handle.emit("app-ready", ());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            parse_videos, read_clipboard, write_clipboard, copy_get_lesson_record_info,
            start_clipboard_watch, stop_clipboard_watch, start_downloads,
            cancel_download, cancel_all_downloads, pause_download, resume_download,
            select_directory, open_folder, get_default_dir, initialize_log,
            read_logs, load_settings, save_settings, get_version,
 show_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}