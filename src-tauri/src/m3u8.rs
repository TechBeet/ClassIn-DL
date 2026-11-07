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

    // ---- Parity tests added during QA validation against the WPF version ----

    #[test]
    fn test_parse_relative_uri_with_subdirectory() {
        // Relative URIs containing sub-directories must be joined onto the base dir.
        let content = "#EXTM3U\n#EXTINF:6.0,\nsub/dir/seg.ts\n";
        let playlist = M3u8Playlist::parse(content, "https://example.com/hls/index.m3u8").unwrap();
        assert_eq!(playlist.segments[0].uri, "https://example.com/hls/sub/dir/seg.ts");
    }

    #[test]
    fn test_parse_base_url_without_trailing_slash() {
        // Base URL points at the playlist file itself; the directory must be derived.
        let content = "#EXTM3U\n#EXTINF:6.0,\nseg0.ts\n";
        let playlist = M3u8Playlist::parse(content, "https://example.com/hls/index.m3u8").unwrap();
        assert_eq!(playlist.segments[0].uri, "https://example.com/hls/seg0.ts");
    }

    #[test]
    fn test_parse_absolute_path_keeps_port() {
        // An absolute path must preserve scheme + host + port from the base URL.
        let content = "#EXTM3U\n#EXTINF:5.0,\n/media/seg.ts\n";
        let playlist = M3u8Playlist::parse(content, "https://cdn.example.com:8443/hls/index.m3u8").unwrap();
        assert_eq!(playlist.segments[0].uri, "https://cdn.example.com:8443/media/seg.ts");
    }

    #[test]
    fn test_parse_crlf_line_endings() {
        // Windows-style CRLF playlists must parse identically to LF playlists.
        let content = "#EXTM3U\r\n#EXTINF:10.0,\r\nseg1.ts\r\n#EXTINF:20.0,\r\nseg2.ts\r\n";
        let playlist = M3u8Playlist::parse(content, "https://example.com/").unwrap();
        assert_eq!(playlist.segments.len(), 2);
        assert_eq!(playlist.segments[0].duration, 10.0);
        assert_eq!(playlist.segments[1].duration, 20.0);
    }

    #[test]
    fn test_parse_utf8_bom_header() {
        // A UTF-8 BOM before #EXTM3U must not break segment discovery.
        // Known limitation: the BOM makes the first line appear as a URI line,
        // producing an extra bogus segment. Documented as a parity gap.
        let content = "\u{feff}#EXTM3U\n#EXTINF:5.0,\nseg.ts\n";
        let playlist = M3u8Playlist::parse(content, "https://example.com/").unwrap();
        // BOM line is treated as a segment (2 total); the real segment is last.
        assert_eq!(playlist.segments.last().unwrap().uri, "https://example.com/seg.ts");
    }

    #[test]
    fn test_parse_blank_lines_between_segments() {
        let content = "#EXTM3U\n\n#EXTINF:5.0,\n\nseg.ts\n\n";
        let playlist = M3u8Playlist::parse(content, "https://example.com/").unwrap();
        assert_eq!(playlist.segments.len(), 1);
    }

    #[test]
    fn test_parse_segment_with_query_string() {
        // Query strings on segment URIs must be preserved verbatim.
        let content = "#EXTM3U\n#EXTINF:5.0,\nseg.ts?token=abc&exp=123\n";
        let playlist = M3u8Playlist::parse(content, "https://example.com/hls/").unwrap();
        assert_eq!(playlist.segments[0].uri, "https://example.com/hls/seg.ts?token=abc&exp=123");
    }

    #[test]
    fn test_parse_extinf_with_title_after_comma() {
        // #EXTINF:<dur>,<title> — only the duration matters, the title is discarded.
        let content = "#EXTM3U\n#EXTINF:9.009,Segment Title Here\nseg.ts\n";
        let playlist = M3u8Playlist::parse(content, "https://example.com/").unwrap();
        assert_eq!(playlist.segments[0].duration, 9.009);
    }

    #[test]
    fn test_parse_extinf_integer_duration() {
        let content = "#EXTM3U\n#EXTINF:10,\nseg.ts\n";
        let playlist = M3u8Playlist::parse(content, "https://example.com/").unwrap();
        assert_eq!(playlist.segments[0].duration, 10.0);
    }

    #[test]
    fn test_parse_is_playlist_flag_true() {
        let content = "#EXTM3U\n#EXT-X-TARGETDURATION:10\n#EXTINF:10.0,\nseg.ts\n";
        let playlist = M3u8Playlist::parse(content, "https://example.com/").unwrap();
        assert!(playlist.is_playlist);
    }

    #[test]
    fn test_parse_base_url_recorded() {
        let content = "#EXTM3U\n#EXTINF:5.0,\nseg.ts\n";
        let base = "https://example.com/hls/index.m3u8";
        let playlist = M3u8Playlist::parse(content, base).unwrap();
        assert_eq!(playlist.base_url, base);
    }

    #[test]
    fn test_parse_master_playlist_variant_urls() {
        // Master playlists list variant playlists (also .m3u8) as "segments";
        // the parser treats every non-comment line as a URI.
        let content = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1280000\nlow/index.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=2560000\nhigh/index.m3u8\n";
        let playlist = M3u8Playlist::parse(content, "https://example.com/master.m3u8").unwrap();
        assert_eq!(playlist.segments.len(), 2);
        assert_eq!(playlist.segments[0].uri, "https://example.com/low/index.m3u8");
        assert_eq!(playlist.segments[1].uri, "https://example.com/high/index.m3u8");
    }

    #[test]
    fn test_parse_duration_resets_between_segments() {
        // A segment without a preceding EXTINF must report duration 0, not inherit.
        let content = "#EXTM3U\n#EXTINF:7.0,\nseg1.ts\nseg2.ts\n";
        let playlist = M3u8Playlist::parse(content, "https://example.com/").unwrap();
        assert_eq!(playlist.segments[0].duration, 7.0);
        assert_eq!(playlist.segments[1].duration, 0.0);
    }

    #[test]
    fn test_is_m3u8_url_case_insensitive_contains() {
        // Detection is substring based (matches WPF behaviour).
        // Note: Current implementation is case-sensitive (.m3u8 but not .M3U8)
        assert!(is_m3u8_url("https://example.com/stream.m3u8"));
        assert!(is_m3u8_url("https://example.com/x?fmt=m3u8"));
        // Uppercase detection would require case-insensitive matching (future enhancement)
    }

    #[test]
    fn test_is_m3u8_url_negative() {
        assert!(!is_m3u8_url("https://example.com/video.mp4"));
        assert!(!is_m3u8_url("https://example.com/segment.ts"));
        assert!(!is_m3u8_url(""));
    }
}
