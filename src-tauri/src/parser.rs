//! Parser module - parses ClassIn video URLs from clipboard content
use regex::Regex;
use lazy_static::lazy_static;
use crate::models::VideoItem;
use anyhow::Result;

lazy_static! {
    /// Match video URLs - supports mp4, m3u8, ts formats
    static ref URL_REGEX: Regex = regex::Regex::new(
        r#"https?://[^\s"'<>{}[\\]+\.(mp4|m3u8|ts)[^\s"'<>{}[\\]*"#
    ).unwrap();
}

/// Parse video list from clipboard content
pub fn parse_videos(content: &str) -> Result<Vec<VideoItem>> {
    let mut videos = Vec::new();
    
    // Try JSON parsing
    if content.contains('{') && content.contains('}') {
        if let Ok(json_obj) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(urls) = extract_urls_from_json(&json_obj) {
                for (i, url) in urls.iter().enumerate() {
                    let name = format_url_name(url, (i + 1) as u32);
                    videos.push(VideoItem::new_named(i as u32 + 1, name, url.clone()));
                }
                if !videos.is_empty() {
                    return Ok(videos);
                }
            }
        }
    }
    
    // Try regex matching
    for cap in URL_REGEX.captures_iter(content) {
        let url = cap.get(0).map(|m| m.as_str()).unwrap_or("");
        if is_valid_url(url) {
            let index = (videos.len() + 1) as u32;
            let name = format_url_name(url, index);
            videos.push(VideoItem::new_named(index, name, url.to_string()));
        }
    }
    
    Ok(videos)
}

/// Extract video name from URL
fn format_url_name(url: &str, default_index: u32) -> String {
    let url_parts: Vec<&str> = url.split('?').collect();
    let path = url_parts[0];
    let filename = path.split('/').last().unwrap_or("");
    
    if let Some(ext_pos) = filename.rfind('.') {
        let name_without_ext = &filename[..ext_pos];
        if name_without_ext.len() > 1 && !name_without_ext.chars().all(|c| c.is_ascii_digit()) {
            return name_without_ext.to_string();
        }
    }
    
    format!("Video_{}", default_index)
}

/// Extract URLs from JSON
fn extract_urls_from_json(value: &serde_json::Value) -> Option<Vec<String>> {
    let mut urls = Vec::new();
    
    if let Some(obj) = value.as_object() {
        for (key, val) in obj {
            if (key.contains("url") || key.contains("video")) && val.is_string() {
                if let Some(s) = val.as_str() {
                    if is_valid_url(s) {
                        urls.push(s.to_string());
                    }
                }
            }
            if val.is_array() {
                if let Some(arr_urls) = extract_urls_from_array(val) {
                    urls.extend(arr_urls);
                }
            } else if val.is_object() {
                if let Some(obj_urls) = extract_urls_from_json(val) {
                    urls.extend(obj_urls);
                }
            }
        }
    }
    
    if !urls.is_empty() {
        Some(urls)
    } else {
        None
    }
}

fn extract_urls_from_array(value: &serde_json::Value) -> Option<Vec<String>> {
    let mut urls = Vec::new();
    if let Some(arr) = value.as_array() {
        for item in arr {
            if let Some(s) = item.as_str() {
                if is_valid_url(s) {
                    urls.push(s.to_string());
                }
            } else if item.is_object() {
                if let Some(obj_urls) = extract_urls_from_json(item) {
                    urls.extend(obj_urls);
                }
            }
        }
    }
    
    if !urls.is_empty() {
        Some(urls)
    } else {
        None
    }
}

fn is_valid_url(url: &str) -> bool {
    url.len() > 20 && url.contains('.') && (url.starts_with("http://") || url.starts_with("https://"))
}
