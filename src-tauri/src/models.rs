//! Models module - data structures for video downloads
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum DownloadStatus {
    #[default]
    Waiting,
    Resolving,
    Downloading,
    Partial,
    Completed,
    Failed,
    Cancelled,
    Paused,
}

/// Status color mapped to a CSS color string at the UI layer.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum StatusColor {
    #[default]
    Gray,
    Orange,
    Green,
    Red,
    Yellow,
}
/// Column widths saved from user preferences
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ColumnWidths {
    pub idx: Option<f64>,
    pub name: Option<f64>,
    pub size: Option<f64>,
    pub progress: Option<f64>,
    pub status: Option<f64>,
    pub action: Option<f64>,
}

/// Sort configuration for the video list
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SortConfig {
    pub column: String,
    pub ascending: bool,
}

/// Application settings persisted to disk
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: String,         // "light" | "dark"
    pub column_widths: ColumnWidths,
    pub sort: SortConfig,
    pub max_concurrent: usize,
    pub download_dir: String,
}

/// A downloaded segment (TS file) state for breakpoint resume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentState {
    pub index: usize,
    pub url: String,
    pub offset: u64,      // already downloaded bytes
    pub total: u64,       // total size of the segment (unknown at start => 0)
}

/// Resume metadata stored in a JSON sidecar file alongside the download
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DownloadResume {
    pub video_id: u32,
    pub url: String,
    pub file_size: u64,
    pub downloaded: u64,
    pub segments: Vec<SegmentState>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoItem {
    pub id: u32,
    pub display_index: String,
    pub name: String,
    pub url: String,
    pub file_size: u64,
    pub downloaded_bytes: u64,
    pub status: String,
    pub status_color: StatusColor,
    pub progress: f64,
    pub current_speed: f64,
    pub remaining_time_secs: u64,
    pub phase: String,
    pub error_message: String,
    // Queue management
    pub priority: i32,
    pub paused: bool,
    pub selected: bool,
    pub drag_order: usize,
}

impl VideoItem {
    pub fn new(index: u32, name: String, url: String) -> Self {
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::SeqCst),
            display_index: format!("{:02}", index),
            name,
            url,
            file_size: 0,
            downloaded_bytes: 0,
            status: "Waiting".to_string(),
            status_color: StatusColor::Gray,
            progress: 0.0,
            current_speed: 0.0,
            remaining_time_secs: 0,
            phase: "Waiting".to_string(),
            error_message: String::new(),
            priority: 0,
            paused: false,
            selected: false,
            drag_order: index as usize,
        }
    }
}

/// Format bytes into human readable string for UI display.
#[allow(dead_code)]
pub fn format_size(bytes: u64) -> String {
    if bytes >= 1 << 30 {
        format!("{:.2} GB", bytes as f64 / (1 << 30) as f64)
    } else if bytes >= 1 << 20 {
        format!("{:.2} MB", bytes as f64 / (1 << 20) as f64)
    } else if bytes >= 1 << 10 {
        format!("{:.2} KB", bytes as f64 / (1 << 10) as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format bytes-per-second into human readable string for UI display.
#[allow(dead_code)]
pub fn format_speed(bps: f64) -> String {
    if bps >= (1 << 30) as f64 {
        format!("{:.2} GB/s", bps / (1 << 30) as f64)
    } else if bps >= (1 << 20) as f64 {
        format!("{:.2} MB/s", bps / (1 << 20) as f64)
    } else if bps >= (1 << 10) as f64 {
        format!("{:.2} KB/s", bps / (1 << 10) as f64)
    } else {
        format!("{:.2} B/s", bps)
    }
}

/// Progress update message emitted from the download manager to the frontend via Tauri IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ProgressUpdate {
    pub video_id: u32,
    pub downloaded: u64,
    pub total: u64,
    pub speed: f64,
    pub remaining_secs: u64,
    pub status: DownloadStatus,
    pub phase: String,
    pub speed_total: f64,    // sum of all download speeds
    pub completed_count: u32,
}

impl ProgressUpdate {
    /// Convenience constructor for a minimal progress update.
    pub fn new(video_id: u32, status: DownloadStatus, phase: &str) -> Self {
        Self {
            video_id,
            downloaded: 0,
            total: 0,
            speed: 0.0,
            remaining_secs: 0,
            status,
            phase: phase.to_string(),
            speed_total: 0.0,
            completed_count: 0,
        }
    }
}

/// Human readable label for a download status (matches the WPF `DisplayStatus`).
/// Kept for reference; frontend uses raw status strings from IPC.
#[allow(dead_code)]
impl DownloadStatus {
    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            DownloadStatus::Waiting => "等待中",
            DownloadStatus::Resolving => "解析中",
            DownloadStatus::Downloading => "下载中",
            DownloadStatus::Partial => "部分完成",
            DownloadStatus::Completed => "已完成",
            DownloadStatus::Failed => "失败",
            DownloadStatus::Cancelled => "已取消",
            DownloadStatus::Paused => "已暂停",
        }
    }
}

/// Format seconds into HH:MM:SS. Kept for reference; frontend computes remaining time locally.
#[allow(dead_code)]
pub fn format_time(seconds: u64) -> String {
    if seconds == u64::MAX {
        return "Unknown".to_string();
    }
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        (seconds % 3600) / 60,
        seconds % 60
    )
}

/// Sanitize a file name so it is safe on all platforms.
pub fn sanitize_filename(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if (c as u32) < 0x20 => '_',
            c => c,
        })
        .collect();
    out = out.trim().trim_matches('.').to_string();
    if out.is_empty() {
        out = "video".to_string();
    }
    if out.len() > 120 {
        out.truncate(120);
    }
    out
}

/// Resolve a video's output filename and directory.
///
/// HLS streams are written as a single `.ts` file (segments concatenated),
/// progressive downloads keep their original container extension.
pub fn resolve_video_path(dir: &str, video: &VideoItem, m3u8: bool) -> String {
    let safe = sanitize_filename(&video.name);
    let stem = match safe.rfind('.') {
        Some(pos) if pos > 0 => &safe[..pos],
        _ => safe.as_str(),
    };
    let ext = if m3u8 { "ts" } else { "mp4" };
    let sep = if dir.ends_with('\\') || dir.ends_with('/') { "" } else { "\\" };
    format!("{dir}{sep}{stem}.{ext}")
}
