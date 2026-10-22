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
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum StatusColor {
    Gray,
    Orange,
    Green,
    Red,
    Yellow,
}

impl Default for StatusColor {
    fn default() -> Self {
        StatusColor::Gray
    }
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
    pub status_flag: DownloadStatus,
    pub progress: f64,
    pub current_speed: f64,
    pub remaining_time_secs: u64,
    pub phase: String,
    pub error_message: String,
    pub is_paused: bool,
    pub max_concurrent: usize,
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
            status: "等待解析".to_string(),
            status_color: StatusColor::Gray,
            status_flag: DownloadStatus::Waiting,
            progress: 0.0,
            current_speed: 0.0,
            remaining_time_secs: 0,
            phase: "等待".to_string(),
            error_message: String::new(),
            is_paused: false,
        }
    }

    pub fn new_named(index: u32, name: String, url: String, max_concurrent: usize) -> Self {
        let mut item = Self::new(index, name, url);
        item.max_concurrent = max_concurrent;
        item
    }
    
    /// 便捷构造函数，使用默认名称 (legacy)
    pub fn new_named_legacy(index: u32, name: String, url: String) -> Self {
        Self::new_named(index, name, url, 5)
    }
}

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

/// Progress update message sent from download manager
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub video_id: u32,
    pub downloaded: u64,
    pub total: u64,
    pub speed: f64,
    pub remaining_secs: u64,
    pub status: DownloadStatus,
}

/// Format seconds into HH:MM:SS
pub fn format_time(seconds: u64) -> String {
    if seconds == u64::MAX {
        return "未知".to_string();
    }
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        (seconds % 3600) / 60,
        seconds % 60
    )
}