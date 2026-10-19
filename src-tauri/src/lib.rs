use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use std::io::Write;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoItem {
    pub id: u32,
    pub name: String,
    pub url: String,
    pub size: u64,
    pub status: String,
    pub progress: f64,
    pub downloaded: u64,
    pub speed: String,
    pub eta: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ParseResult {
    pub videos: Vec<VideoItem>,
    pub course_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DownloadRequest {
    pub videos: Vec<VideoItem>,
    pub output_dir: String,
    pub max_concurrent: u32,
    pub max_threads: u32,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            parse_video_info,
            download_video,
            get_clipboard_text,
            set_clipboard_text,
            get_download_dir,
            select_download_dir,
            open_download_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
#[tauri::command]
fn parse_video_info(content: String) -> Result<ParseResult, String> {
    use serde_json::Value;
    
    let json: Value = serde_json::from_str(&content)
        .map_err(|e| format!("JSON parse error: {}", e))?;
    
    let course_name = json.get("lessonName")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown Course")
        .to_string();
    
    let file_list = json.get("fileList")
        .and_then(|v| v.as_array())
        .ok_or("No fileList found")?;
    
    let mut videos = Vec::new();
    
    for (i, file) in file_list.iter().enumerate() {
        let name = file.get("title")
            .or_else(|| file.get("fileName"))
            .and_then(|v| v.as_str())
            .unwrap_or(&format!("Video{}", i + 1))
            .to_string();
        
        let playset = file.get("Playset")
            .and_then(|v| v.as_array())
            .ok_or("No Playset found")?;
        
        let mut url = String::new();
        for item in playset.iter().rev() {
            if let Some(u) = item.get("URL").and_then(|v| v.as_str()) {
                if !u.is_empty() {
                    url = u.to_string();
                    break;
                }
            }
        }
        
        if url.is_empty() { continue; }
        
        let size = file.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
        
        videos.push(VideoItem {
            id: i as u32 + 1,
            name: sanitize_filename(&name),
            url,
            size,
            status: "Pending".to_string(),
            progress: 0.0,
            downloaded: 0,
            speed: "0 B/s".to_string(),
            eta: "--:--".to_string(),
        });
    }
    
    Ok(ParseResult { videos, course_name })
}

#[tauri::command]
fn download_video(request: DownloadRequest) -> Result<(), String> {
    let output_dir = &request.output_dir;
    
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Create dir error: {}", e))?;
    
    for video in &request.videos {
        download_single_video(video, output_dir)?;
    }
    
    Ok(())
}

fn download_single_video(video: &VideoItem, output_dir: &str) -> Result<(), String> {
    use reqwest::blocking::Client;
    
    let client = Client::new();
    let filepath = PathBuf::from(output_dir).join(format!("{}.mp4", video.name));
    
    if filepath.exists() { return Ok(()); }
    
    let response = client.head(&video.url)
        .send()
        .map_err(|e| format!("Request failed: {}", e))?;
    
    let _total_size = response.content_length().unwrap_or(video.size);
    let mut file = fs::File::create(&filepath)
        .map_err(|e| format!("Create file error: {}", e))?;
    
    let response = client.get(&video.url).send()
        .map_err(|e| format!("Request failed: {}", e))?;
    
    let bytes = response.bytes()
        .map_err(|e| format!("Failed to read: {}", e))?;
    
    file.write_all(&bytes)
        .map_err(|e| format!("Write error: {}", e))?;
    
    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    name.replace("/", "_").replace("\\", "_").replace(":", "_").replace("*", "_")
        .replace("?", "_").replace("\"", "_").replace("<", "_").replace(">", "_")
        .replace("|", "_")
}

#[tauri::command]
fn get_clipboard_text() -> Result<String, String> {
    use arboard::Clipboard;
    let mut clipboard = Clipboard::new()
        .map_err(|e| format!("Clipboard error: {}", e))?;
    let text = clipboard.get_text()
        .map_err(|e| format!("Clipboard read error: {}", e))?;
    Ok(text)
}

#[tauri::command]
fn set_clipboard_text(text: String) -> Result<(), String> {
    use arboard::Clipboard;
    let mut clipboard = Clipboard::new()
        .map_err(|e| format!("Clipboard error: {}", e))?;
    clipboard.set_text(text)
        .map_err(|e| format!("Clipboard write error: {}", e))?;
    Ok(())
}

#[tauri::command]
fn get_download_dir() -> Result<String, String> {
    let home_dir = dirs::document_dir()
        .or_else(|| dirs::home_dir())
        .ok_or("Cannot get home directory")?;
    let download_dir = home_dir.join("Downloads");
    Ok(download_dir.to_string_lossy().to_string())
}

#[tauri::command]
fn select_download_dir() -> Result<String, String> {
    let folder = rfd::FileDialog::new()
        .set_title("Select Download Directory")
        .pick_folder()
        .ok_or("User cancelled")?;
    Ok(folder.to_string_lossy().to_string())
}

#[tauri::command]
fn open_download_dir(path: String) -> Result<(), String> {
    open::that(path)
        .map_err(|e| format!("Open error: {}", e))?;
    Ok(())
}
