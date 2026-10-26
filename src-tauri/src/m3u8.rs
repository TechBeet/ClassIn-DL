//! M3U8 module - handles HLS playlist parsing and segment downloads
use anyhow::Result;

/// A single TS segment in an HLS playlist
#[derive(Debug, Clone)]
pub struct TsSegment {
    pub uri: String,
    #[allow(dead_code)]
    pub duration: f64,
}

/// Parsed HLS playlist
#[derive(Debug)]
pub struct M3u8Playlist {
    pub segments: Vec<TsSegment>,
    #[allow(dead_code)]
    pub base_url: String,
    #[allow(dead_code)]
    pub is_playlist: bool,
}

impl M3u8Playlist {
    /// Parse M3U8 content into a playlist structure
    pub fn parse(content: &str, base_url: &str) -> Result<Self> {
        let mut segments = Vec::new();
        let mut current_duration = 0.0;
        
        // Extract base directory from the URL
        let url_base = if base_url.contains('/') {
            base_url[..base_url.rfind('/').unwrap_or(base_url.len())].to_string()
        } else {
            base_url.to_string()
        };
        
        let is_playlist = content.starts_with("#EXTM3U") || content.contains("#EXT-X-TARGETDURATION");
        
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') && !line.starts_with("#EXTINF:") {
                if line.starts_with("#EXTINF:") {
                    if let Some(duration_str) = line.strip_prefix("#EXTINF:") {
                        if let Some(d_str) = duration_str.split(',').next() {
                            if let Ok(d) = d_str.parse::<f64>() {
                                current_duration = d;
                            }
                        }
                    }
                }
                continue;
            }
            
            // Resolve relative URI
            let uri = if line.starts_with("http://") || line.starts_with("https://") {
                line.to_string()
            } else if let Some(stripped) = line.strip_prefix('/') {
                // Absolute path - need the scheme and host from base_url
                let scheme_end = url_base.find("://").unwrap_or(url_base.len());
                let rest = &url_base[scheme_end + 3..];
                let host_end = rest.find('/').unwrap_or(rest.len());
                format!("{}://{}/{}", &url_base[..scheme_end], &rest[..host_end], stripped)
            } else {
                format!("{}/{}", url_base, line)
            };
            
            segments.push(TsSegment {
                uri,
                duration: current_duration,
            });
            current_duration = 0.0;
        }
        
        if segments.is_empty() {
            return Err(anyhow::anyhow!("No segments found in M3U8 playlist"));
        }
        
        Ok(M3u8Playlist {
            segments,
            base_url: base_url.to_string(),
            is_playlist,
        })
    }
}

/// Detect if a URL is an M3U8 stream
pub fn is_m3u8_url(url: &str) -> bool {
    url.contains(".m3u8") || url.contains("m3u8")
}
