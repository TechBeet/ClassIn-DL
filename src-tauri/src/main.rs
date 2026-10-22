// ClassIn Video Downloader - Tauri 2.x Backend
mod models;
mod clipboard;
mod parser;
mod downloader;
mod m3u8;

use tauri::Manager;
use std::sync::{Arc, Mutex};

#[derive(Clone, serde::Serialize)]
struct Payload {
    id: u32,
    url: String,
    name: String,
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize download manager
            let manager = Arc::new(Mutex::new(downloader::DownloadManager::new(5, 32)));
            app.manage(manager);
            
            // Run on main thread for Windows clipboard access
            #[cfg(windows)]
            app.handle().inner().dispatch_event(winit::event::Event::NewEvents(
                winit::event::StartCause::Init
            ));
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Parser commands
            parser_commands::parse_videos,
            parser_commands::read_clipboard,
            parser_commands::write_clipboard,
            parser_commands::copy_get_lesson_record_info,
            
            // Download commands
            download_commands::start_downloads,
            download_commands::cancel_download,
            download_commands::cancel_all_downloads,
            download_commands::get_progress,
            
            // Directory commands
            dir_commands::select_directory,
            dir_commands::open_folder,
            dir_commands::get_default_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Parser commands module
mod parser_commands {
    use super::*;
    use tauri::command;
    
    #[command]
    pub async fn parse_videos(content: &str, max_concurrent: usize) -> Result<Vec<models::VideoItem>, String> {
        parser::parse_videos(content, max_concurrent).await
    }
    
    #[command]
    pub async fn read_clipboard() -> Result<String, String> {
        clipboard::read_clipboard().await
    }
    
    #[command]
    pub async fn write_clipboard(text: &str) -> Result<(), String> {
        clipboard::write_clipboard(text).await
    }
    
    #[command]
    pub async fn copy_get_lesson_record_info() -> Result<(), String> {
        clipboard::copy_get_lesson_record_info().await
    }
}

// Download commands module
mod download_commands {
    use super::*;
    use tauri::command;
    
    #[command]
    pub async fn start_downloads(
        videos: Vec<models::VideoItem>,
        dir: &str,
        max_concurrent: usize,
    ) -> Result<(), String> {
        let manager = Arc::clone(&tauri::State::<Arc<Mutex<downloader::DownloadManager>>>::get_state()?);
        let mut mgr = manager.lock().unwrap();
        mgr.start_downloads(videos, dir, max_concurrent).await
    }
    
    #[command]
    pub async fn cancel_download(id: u32) -> Result<(), String> {
        let manager = Arc::clone(&tauri::State::<Arc<Mutex<downloader::DownloadManager>>>::get_state()?);
        let mut mgr = manager.lock().unwrap();
        mgr.cancel_download(id)
    }
    
    #[command]
    pub async fn cancel_all_downloads() -> Result<(), String> {
        let manager = Arc::clone(&tauri::State::<Arc<Mutex<downloader::DownloadManager>>>::get_state()?);
        let mut mgr = manager.lock().unwrap();
        mgr.cancel_all()
    }
    
    #[command]
    pub async fn get_progress() -> Result<Vec<models::ProgressUpdate>, String> {
        let manager = Arc::clone(&tauri::State::<Arc<Mutex<downloader::DownloadManager>>>::get_state()?);
        let mgr = manager.lock().unwrap();
        Ok(mgr.get_progress())
    }
}

// Directory commands module
mod dir_commands {
    use super::*;
    use tauri::command;
    
    #[command]
    pub async fn select_directory() -> Result<String, String> {
        rfd::AsyncFileDialog::new()
            .set_title("选择下载目录")
            .pick_folder()
            .await
            .map(|f| f.path().to_string_lossy().to_string())
            .map_err(|e| e.to_string())
    }
    
    #[command]
    pub async fn open_folder(path: &str) -> Result<(), String> {
        open::that(path).map_err(|e| e.to_string())
    }
    
    #[command]
    pub fn get_default_dir() -> Result<String, String> {
        Ok(dirs::home_dir()
            .map(|d| d.join("ClassInDownloads").to_string_lossy().to_string())
            .unwrap_or_else(|| "ClassInDownloads".to_string()))
    }
}

const DEFAULT_MAX_CONCURRENT: usize = 5;
const DEFAULT_MAX_THREADS: usize = 32;
const MAX_LOG_ENTRIES: usize = 100;

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum Message {
    ParseVideos,
    SelectDir(Option<String>),
    StartDownload,
    CancelDownload(u32),
    CancelAllDownloads,
    DeleteVideo(u32),
    CopyCommand,
    OpenFolder,
    UpdateProgress(models::ProgressUpdate),
    AddLog(String),
    ClearError,
    PollProgress,
    UpdateMaxConcurrent(usize),
}

#[derive(Debug, Clone, Default)]
struct State {
    videos: Vec<models::VideoItem>,
    output_dir: String,
    error_message: String,
    logs: Vec<String>,
    active_download_count: usize,
    completed_count: usize,
    failed_count: usize,
    max_concurrent: usize,
    total_speed: f64,
}

pub struct App {
    state: State,
    download_manager: Arc<TokioMutex<downloader::DownloadManager>>,
    progress_tx: mpsc::Sender<models::ProgressUpdate>,
    progress_rx: Arc<TokioMutex<mpsc::Receiver<models::ProgressUpdate>>>,
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ParseVideos => {
                self.handle_parse_videos();
                Task::done(Message::AddLog("Parsing...".to_string()))
            }
            Message::SelectDir(dir) => {
                if let Some(d) = dir {
                    self.state.output_dir = d.clone();
                    self.state.logs.push(format!("Selected folder: {}", d));
                }
                Task::none()
            }
            Message::StartDownload => {
                self.handle_start_download();
                Task::none()
            }
            Message::CancelDownload(id) => {
                self.handle_cancel_download(id);
                Task::none()
            }
            Message::DeleteVideo(id) => {
                self.handle_delete_video(id);
                Task::none()
            }
            Message::CopyCommand => {
                self.handle_copy_command();
                Task::none()
            }
            Message::UpdateProgress(update) => {
                self.handle_progress_update(update);
                Task::none()
            }
            Message::AddLog(msg) => {
                self.state.logs.push(msg);
                if self.state.logs.len() > MAX_LOG_ENTRIES {
                    self.state.logs.remove(0);
                }
                Task::none()
            }
            Message::ClearError => {
                self.state.error_message.clear();
                Task::none()
            }
            Message::PollProgress => {
                self.poll_progress()
            }
            Message::CancelAllDownloads => {
                self.handle_cancel_all();
                Task::none()
            }
            Message::OpenFolder => {
                self.handle_open_folder();
                Task::none()
            }
            Message::UpdateMaxConcurrent(val) => {
                self.state.max_concurrent = val;
                Task::none()
            }
        }
    }

    fn handle_parse_videos(&mut self) {
        self.state.error_message.clear();
        self.state.logs.clear();
        let clipboard_text = match clipboard::read_clipboard() {
            Ok(text) => text,
            Err(e) => {
                self.state.error_message = format!("Failed to read clipboard: {}", e);
                return;
            }
        };
        let videos = match parser::parse_videos(&clipboard_text) {
            Ok(videos) => videos,
            Err(e) => {
                self.state.error_message = format!("Failed to parse videos: {}", e);
                return;
            }
        };
        self.state.videos = videos;
        self.state.logs.push(format!("Parsed {} videos", self.state.videos.len()));
    }

    fn handle_start_download(&mut self) {
        if self.state.videos.is_empty() {
            self.state.error_message = "No videos to download".to_string();
            return;
        }
        if self.state.output_dir.is_empty() {
            self.state.error_message = "Please select a download folder".to_string();
            return;
        }
        self.state.active_download_count = self.state.videos.len();
        self.state.completed_count = 0;
        self.state.failed_count = 0;
        self.state.total_speed = 0.0;
        for video in &mut self.state.videos {
            video.status_flag = models::DownloadStatus::Waiting;
            video.status_color = models::StatusColor::Gray;
            video.status = "Waiting".to_string();
            video.progress = 0.0;
            video.downloaded_bytes = 0;
            video.current_speed = 0.0;
            video.remaining_time_secs = 0;
        }
        let manager = self.download_manager.clone();
        let output_dir = self.state.output_dir.clone();
        let videos: Vec<(u32, String, String)> = self.state.videos.iter().map(|v| (v.id, v.url.clone(), v.display_index.clone())).collect();
        let progress_tx = self.progress_tx.clone();
        self.state.logs.push(format!("Starting {} downloads...", videos.len()));
        tokio::spawn(async move {
            let mgr = manager.lock().await;
            for (vid, url, display_index) in videos {
                let output_path = format!("{}/{}.mp4", output_dir, display_index);
                if let Err(e) = mgr.start_download(vid, url, output_path, progress_tx.clone()).await {
                    eprintln!("Download failed for {}: {}", vid, e);
                }
            }
        });
    }

    fn handle_cancel_download(&mut self, video_id: u32) {
        let manager = self.download_manager.clone();
        tokio::spawn(async move {
            let mgr = manager.lock().await;
            mgr.cancel_download(video_id).await;
        });
    }

    fn handle_delete_video(&mut self, video_id: u32) {
        if let Some(idx) = self.state.videos.iter().position(|v| v.id == video_id) {
            self.state.videos.remove(idx);
        }
    }

    fn handle_copy_command(&mut self) {
        if let Err(e) = clipboard::copy_get_lesson_record_info_command() {
            self.state.error_message = format!("Failed to copy command: {}", e);
        } else {
            self.state.logs.push("Command copied to clipboard".to_string());
        }
    }

    fn handle_cancel_all(&mut self) {
        let manager = self.download_manager.clone();
        tokio::spawn(async move {
            let mgr = manager.lock().await;
            mgr.cancel_all().await;
        });
        self.state.logs.push("All downloads cancelled".to_string());
    }

    fn handle_open_folder(&mut self) {
        if !self.state.output_dir.is_empty() {
            downloader::DownloadManager::open_download_folder(&self.state.output_dir);
        }
    }

    fn poll_progress(&mut self) -> Task<Message> {
        // Clone the receiver to avoid borrow conflicts
        let rx_clone = Arc::clone(&self.progress_rx);
        match rx_clone.try_lock() {
            Ok(mut rx) => {
                let mut updates = Vec::new();
                while let Ok(update) = rx.try_recv() {
                    updates.push(update);
                }
                for update in updates {
                    self.handle_progress_update(update);
                }
            }
            Err(_) => {}
        }
        Task::none()
    }

    fn handle_progress_update(&mut self, update: models::ProgressUpdate) {
        if let Some(video) = self.state.videos.iter_mut().find(|v| v.id == update.video_id) {
            video.downloaded_bytes = update.downloaded;
            video.file_size = update.total;
            video.current_speed = update.speed;
            video.remaining_time_secs = update.remaining_secs;
            video.progress = if update.total > 0 {
                update.downloaded as f64 / update.total as f64
            } else {
                0.0
            };
            match update.status {
                models::DownloadStatus::Downloading => {
                    video.status_flag = models::DownloadStatus::Downloading;
                    video.status_color = models::StatusColor::Orange;
                    video.status = format!("{} ({}) - ETA: {}",
                        models::format_speed(update.speed),
                        models::format_size(update.downloaded),
                        models::format_time(update.remaining_secs));
                }
                models::DownloadStatus::Completed => {
                    video.status_flag = models::DownloadStatus::Completed;
                    video.status_color = models::StatusColor::Green;
                    video.status = "Done".to_string();
                    video.progress = 1.0;
                    self.state.completed_count += 1;
                    if self.state.active_download_count > 0 {
                        self.state.active_download_count = self.state.active_download_count.saturating_sub(1);
                    }
                }
                models::DownloadStatus::Failed => {
                    video.status_flag = models::DownloadStatus::Failed;
                    video.status_color = models::StatusColor::Red;
                    video.status = "Failed".to_string();
                    self.state.failed_count += 1;
                    if self.state.active_download_count > 0 {
                        self.state.active_download_count = self.state.active_download_count.saturating_sub(1);
                    }
                }
                models::DownloadStatus::Cancelled => {
                    video.status_flag = models::DownloadStatus::Cancelled;
                    video.status_color = models::StatusColor::Gray;
                    video.status = "Cancelled".to_string();
                    if self.state.active_download_count > 0 {
                        self.state.active_download_count = self.state.active_download_count.saturating_sub(1);
                    }
                }
                _ => {}
            }
        }
        self.state.total_speed = update.speed;
    }
}

fn subscription(_app: &App) -> Subscription<Message> {
    use iced::time::every;
    every(std::time::Duration::from_millis(200)).map(|_| Message::PollProgress)
}

fn view(app: &App) -> Element<'_, Message> {
    let state = &app.state;
    let header = container(text("ClassIn Video Downloader").size(24)).padding(16);
    let toolbar = container(
        row![
            button("Parse Videos").on_press(Message::ParseVideos),
            button("Select Folder").on_press(Message::SelectDir(None)),
            button("Start Download").on_press(Message::StartDownload),
            button("Cancel All").on_press(Message::CancelAllDownloads),
            button("Open Folder").on_press(Message::OpenFolder),
            button("Copy Command").on_press(Message::CopyCommand),
            button("Clear Error").on_press(Message::ClearError),
        ].spacing(10),
    ).padding(12);
    let max_concurrent_row = row![
        text("Max Concurrent:").width(Length::Fixed(130.0)),
        text_input("5", &state.max_concurrent.to_string())
            .on_input(|s: String| s.parse().map(Message::UpdateMaxConcurrent).unwrap_or(Message::UpdateMaxConcurrent(1))),
        text("concurrent downloads").width(Length::Fixed(180.0)),
    ];
    let dir_info = container(
        row![
            text("Download Folder:").width(Length::Fixed(100.0)),
            text(&state.output_dir).width(Length::Fill),
        ].padding(12),
    );
    let error_box: Element<'_, Message> = if !state.error_message.is_empty() {
        container(text(&state.error_message).size(14)).padding(12).into()
    } else {
        Space::with_height(Length::Fixed(0.0)).into()
    };

    let stats_row = row![
        text(format!("Total: {} videos", state.videos.len())).width(Length::Fixed(120.0)),
        text(format!("Done: {}", state.completed_count)).width(Length::Fixed(80.0)),
        text(format!("Active: {}", state.active_download_count)).width(Length::Fixed(80.0)),
        text(format!("Failed: {}", state.failed_count)).width(Length::Fixed(80.0)),
        text(format!("Speed: {}", models::format_speed(state.total_speed))).width(Length::Fixed(120.0)),
    ];
    let video_list = container(
        column(
            state.videos.iter().map(|video| {
                let status_text = match video.status_flag {
                    models::DownloadStatus::Waiting => "Waiting".to_string(),
                    models::DownloadStatus::Downloading => {
                        format!("{} ({}) - ETA: {}",
                            models::format_speed(video.current_speed),
                            models::format_size(video.downloaded_bytes),
                            models::format_time(video.remaining_time_secs))
                    }
                    models::DownloadStatus::Completed => "Done".to_string(),
                    models::DownloadStatus::Failed => "Failed".to_string(),
                    models::DownloadStatus::Cancelled => "Cancelled".to_string(),
                    _ => video.status.clone(),
                };
                row![
                    text(&video.display_index).width(Length::Fixed(60.0)).align_x(Alignment::Center),
                    text(&video.name).width(Length::Fill).align_x(Alignment::Start),
                    text(models::format_size(video.file_size)).width(Length::Fixed(100.0)).align_x(Alignment::End),
                    ProgressBar::new(0.0..=1.0, video.progress as f32).width(Length::Fixed(200.0)),
                    text(status_text).width(Length::Fixed(120.0)).align_x(Alignment::Center),
                    row![
                        button("Cancel").on_press(Message::CancelDownload(video.id)),
                        button("Delete").on_press(Message::DeleteVideo(video.id)),
                    ].spacing(5),
                ].spacing(10).padding(8).into()
            }).collect::<Vec<_>>(),
        ).spacing(5),
    );
    let log_section = container(
        column![
            text("Log").size(14),
            scrollable(
                column(
                    state.logs.iter().rev().take(100).map(|log| text(log).size(12).into()).collect::<Vec<_>>(),
                ).spacing(2).padding(10),
            ),
        ].spacing(10),
    );
    let content = column![
        header, toolbar, max_concurrent_row, stats_row, dir_info,
        container(error_box).width(Length::Fill),
        video_list, log_section,
    ].spacing(10).padding(10);
    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .into()
}

fn main() -> iced::Result {
    let (progress_tx, progress_rx) = mpsc::channel(100);
    let manager = Arc::new(TokioMutex::new(downloader::DownloadManager::new(
        DEFAULT_MAX_CONCURRENT,
        DEFAULT_MAX_THREADS,
    )));
    let output_dir = dirs::home_dir()
        .map(|d| d.join("ClassInDownloads").to_string_lossy().to_string())
        .unwrap_or_else(|| "ClassInDownloads".to_string());
    let state = State {
        output_dir,
        max_concurrent: DEFAULT_MAX_CONCURRENT,
        ..Default::default()
    };
    iced::application("ClassIn Video Downloader", App::update, view)
        .subscription(subscription)
        .run_with(|| (App { state, download_manager: manager, progress_tx, progress_rx: Arc::new(TokioMutex::new(progress_rx)) }, Task::none()))
}