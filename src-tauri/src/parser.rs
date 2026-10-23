//! Parser module - parses ClassIn video URLs from clipboard content
use regex::Regex;
use lazy_static::lazy_static;
use crate::models::VideoItem;
use anyhow::Result;

lazy_static! {
    static ref URL_REGEX: Regex = regex::Regex::new(
        r#"https?://[^\s"'<>\{\}\[\]\\]+\.(mp4|m3u8|ts)[^\s"'<>\{\}\[\]\\]*"#
    ).unwrap();
}

pub async fn parse_videos(content: &str, max_concurrent: usize) -> Result<Vec<VideoItem>> {
    let mut videos = Vec::new();
    if content.contains('{') && content.contains('}') {
        if let Ok(json_obj) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(parsed) = extract_videos_from_json(&json_obj, max_concurrent) {
                return Ok(parsed);
            }
        }
    }
    for cap in URL_REGEX.captures_iter(content) {
        let url = cap.get(0).map(|m| m.as_str()).unwrap_or("");
        if is_valid_url(url) {
            let index = (videos.len() + 1) as u32;
            let name = format_url_name(url, index);
            videos.push(VideoItem::new(index, name, url.to_string()));
        }
    }
    Ok(videos)
}

fn extract_videos_from_json(value: &serde_json::Value, _mc: usize) -> Option<Vec<VideoItem>> {
    let mut videos = Vec::new();
    if let Some(obj) = value.as_object() {
        let lesson_name = obj.iter()
            .find(|(k, _)| *k == "lessonName")
            .map(|(_, v)| v.as_str().unwrap_or("Unknown"))
            .unwrap_or("Unknown");
        for (key, val) in obj {
            if (key == "url" || key == "video" || key == "link") && val.is_string() {
                if let Some(s) = val.as_str() {
                    if is_valid_url(s) {
                        let index = (videos.len() + 1) as u32;
                        videos.push(VideoItem::new(index, format!("{}_{}", lesson_name, index), s.to_string()));
                    }
                }
            }
            if key == "Playset" || key == "videos" || key == "urls" {
                if let Some(arr) = val.as_array() {
                    for item in arr.iter() {
                        if item.is_string() {
                            if let Some(s) = item.as_str() {
                                if is_valid_url(s) {
                                    let index = (videos.len() + 1) as u32;
                                    videos.push(VideoItem::new(index, format!("{}_{}", lesson_name, index), s.to_string()));
                                }
                            }
                        } else if item.is_object() {
                            if let Some(nested) = extract_url_from_object(item) {
                                let index = (videos.len() + 1) as u32;
                                videos.push(VideoItem::new(index, format!("{}_{}", lesson_name, index), nested));
                            }
                        }
                    }
                }
            }
            if val.is_array() {
                if let Some(arr) = extract_urls_from_array(val, lesson_name) {
                    videos.extend(arr);
                }
            }
            if val.is_object() {
                if let Some(nested) = extract_videos_from_json(val, _mc) {
                    videos.extend(nested);
                }
            }
        }
    }
    if !videos.is_empty() { Some(videos) } else { None }
}

fn extract_url_from_object(obj: &serde_json::Value) -> Option<String> {
    if let Some(map) = obj.as_object() {
        for key in ["url", "video", "link", "src"] {
            if let Some(val) = map.get(key) {
                if let Some(s) = val.as_str() {
                    if is_valid_url(s) { return Some(s.to_string()); }
                }
            }
        }
    }
    None
}

fn extract_urls_from_array(value: &serde_json::Value, lesson_name: &str) -> Option<Vec<VideoItem>> {
    let mut videos = Vec::new();
    if let Some(arr) = value.as_array() {
        for item in arr {
            if let Some(s) = item.as_str() {
                if is_valid_url(s) {
                    let index = (videos.len() + 1) as u32;
                    videos.push(VideoItem::new(index, format!("{}_{}", lesson_name, index), s.to_string()));
                }
            } else if item.is_object() {
                if let Some(url) = extract_url_from_object(item) {
                    let index = (videos.len() + 1) as u32;
                    videos.push(VideoItem::new(index, format!("{}_{}", lesson_name, index), url));
                }
            }
        }
    }
    if !videos.is_empty() { Some(videos) } else { None }
}

fn format_url_name(url: &str, default_index: u32) -> String {
    let path = url.split('?').collect::<Vec<&str>>()[0];
    let filename = path.split('/').last().unwrap_or("");
    if let Some(ext_pos) = filename.rfind('.') {
        let name = &filename[..ext_pos];
        if name.len() > 1 && !name.chars().all(|c| c.is_ascii_digit()) {
            return name.to_string();
        }
    }
    format!("Video_{}", default_index)
}

fn is_valid_url(url: &str) -> bool {
    url.len() > 20 && url.contains('.') && (url.starts_with("http://") || url.starts_with("https://"))
}