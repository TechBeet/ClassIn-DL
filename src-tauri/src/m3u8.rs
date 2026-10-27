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
            if line.is_empty() {
                continue;
            }
            if line.starts_with('#') {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_playlist() {
        let content = "#EXTM3U\n#EXTINF:10.0,\nsegment1.ts\n#EXTINF:20.0,\nsegment2.ts\n";
        let playlist = M3u8Playlist::parse(content, "https://example.com/path/").unwrap();
        assert_eq!(playlist.segments.len(), 2);
        assert_eq!(playlist.segments[0].uri, "https://example.com/path/segment1.ts");
        assert_eq!(playlist.segments[0].duration, 10.0);
        assert_eq!(playlist.segments[1].uri, "https://example.com/path/segment2.ts");
        assert_eq!(playlist.segments[1].duration, 20.0);
    }

    #[test]
    fn test_parse_absolute_paths() {
        let content = "#EXTM3U\n#EXTINF:5.0,\n/absolute/path/segment.ts\n";
        let playlist = M3u8Playlist::parse(content, "https://example.com/path/").unwrap();
        assert_eq!(playlist.segments[0].uri, "https://example.com/absolute/path/segment.ts");
    }

    #[test]
    fn test_parse_full_urls() {
        let content = "#EXTM3U\n#EXTINF:10.0,\nhttps://cdn.example.com/segment.ts\n";
        let playlist = M3u8Playlist::parse(content, "https://example.com/path/").unwrap();
        assert_eq!(playlist.segments[0].uri, "https://cdn.example.com/segment.ts");
    }

    #[test]
    fn test_parse_empty_playlist() {
        let content = "#EXTM3U\n";
        let result = M3u8Playlist::parse(content, "https://example.com/path/");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_no_segments() {
        let content = "#EXTM3U\n#EXT-X-TARGETDURATION:10\n";
        let result = M3u8Playlist::parse(content, "https://example.com/path/");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_multiple_extinf_lines() {
        let content = "#EXTM3U\n#EXTINF:5.0,\nseg1.ts\n#EXTINF:10.0,\n# comment\nseg2.ts\n#EXTINF:15.0,\nseg3.ts\n";
        let playlist = M3u8Playlist::parse(content, "https://example.com/").unwrap();
        assert_eq!(playlist.segments.len(), 3);
        assert_eq!(playlist.segments[0].duration, 5.0);
        assert_eq!(playlist.segments[1].duration, 10.0);
        assert_eq!(playlist.segments[2].duration, 15.0);
    }

    #[test]
    fn test_parse_no_extm3u_header() {
        let content = "segment1.ts\nsegment2.ts\n";
        let playlist = M3u8Playlist::parse(content, "https://example.com/").unwrap();
        assert_eq!(playlist.segments.len(), 2);
        assert!(!playlist.is_playlist);
    }

    #[test]
    fn test_is_m3u8_url() {
        assert!(is_m3u8_url("https://example.com/stream.m3u8"));
        assert!(is_m3u8_url("https://example.com/stream.m3u8?token=abc"));
        assert!(!is_m3u8_url("https://example.com/video.mp4"));
    }

    #[test]
    fn test_parse_duration_decimal() {
        let content = "#EXTM3U\n#EXTINF:10.5,\nsegment.ts\n";
        let playlist = M3u8Playlist::parse(content, "https://example.com/").unwrap();
        assert_eq!(playlist.segments[0].duration, 10.5);
    }

    #[test]
    fn test_parse_with_comments() {
        let content = "#EXTM3U\n# comment line\n#EXTINF:5.0,\nsegment.ts\n";
        let playlist = M3u8Playlist::parse(content, "https://example.com/").unwrap();
        assert_eq!(playlist.segments.len(), 1);
    }
}
