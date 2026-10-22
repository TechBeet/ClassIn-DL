use crate::models::{DownloadStatus, ProgressUpdate};
use tokio::sync::mpsc;
use anyhow::Result;
use tokio::io::AsyncWriteExt;
use futures_util::stream::StreamExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex as TokioMutex;
use open::that as open_folder;

/// Download task with cancellation support
#[allow(dead_code)]
pub struct DownloadTask {
    pub video_id: u32,
    pub cancelled: Arc<AtomicBool>,
}

/// HLS (m3u8) playlist parser
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TsSegment {
    uri: String,
    duration: f64,
}

#[derive(Debug)]
#[allow(dead_code)]
struct M3u8Playlist {
    segments: Vec<TsSegment>,
    base_url: String,
}

impl M3u8Playlist {
    fn parse(content: &str, base_url: &str) -> Option<Self> {
        let mut segments = Vec::new();
        let mut current_duration = 0.0;
        let url_base = if base_url.contains('/') {
            base_url.rsplit('/').next().unwrap_or("").to_string()
        } else {
            String::new()
        };
        
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
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
            
            let uri = if line.starts_with("http") {
                line.to_string()
            } else if line.starts_with('/') {
                format!("{}{}", url_base, line)
            } else {
                format!("{}/{}", base_url, line)
            };
            
            segments.push(TsSegment {
                uri,
                duration: current_duration,
            });
            current_duration = 0.0;
        }
        
        if !segments.is_empty() {
            Some(M3u8Playlist {
                segments,
                base_url: base_url.to_string(),
            })
        } else {
            None
        }
    }
}

/// Progress tracker with global cancellation support
#[allow(dead_code)]
pub struct DownloadManager {
    max_concurrent: usize,
    max_threads: usize,
    tasks: Arc<TokioMutex<HashMap<u32, DownloadTask>>>,
    global_cancelled: Arc<AtomicBool>,
    active_count: Arc<TokioMutex<usize>>,
}

impl DownloadManager {
    #[allow(dead_code)]
    pub fn new(max_concurrent: usize, max_threads: usize) -> Self {
        Self {
            max_concurrent,
            max_threads,
            tasks: Arc::new(TokioMutex::new(HashMap::new())),
            global_cancelled: Arc::new(AtomicBool::new(false)),
            active_count: Arc::new(TokioMutex::new(0)),
        }
    }

    #[allow(dead_code)]
    pub async fn get_active_count(&self) -> usize {
        *self.active_count.lock().await
    }

    pub async fn cancel_all(&self) {
        self.global_cancelled.store(true, Ordering::SeqCst);
        let mut tasks = self.tasks.lock().await;
        for task in tasks.values_mut() {
            task.cancelled.store(true, Ordering::SeqCst);
        }
    }

    #[allow(dead_code)]
    pub async fn reset_cancellation(&self) {
        self.global_cancelled.store(false, Ordering::SeqCst);
        let mut tasks = self.tasks.lock().await;
        for task in tasks.values_mut() {
            task.cancelled.store(false, Ordering::SeqCst);
        }
    }

    #[allow(dead_code)]
    async fn download_m3u8(
        &self,
        video_id: u32,
        url: &str,
        output_path: &str,
        progress_tx: &mpsc::Sender<ProgressUpdate>,
        client: &reqwest::Client,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<()> {
        let playlist_resp = match client.get(url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                progress_tx.send(ProgressUpdate {
                    video_id, downloaded: 0, total: 0, speed: 0.0,
                    remaining_secs: 0, status: DownloadStatus::Failed,
                }).await?;
                return Err(e.into());
            }
        };
        let playlist_content = playlist_resp.text().await?;

        let playlist = match M3u8Playlist::parse(&playlist_content, url) {
            Some(p) => p,
            None => {
                progress_tx.send(ProgressUpdate {
                    video_id, downloaded: 0, total: 0, speed: 0.0,
                    remaining_secs: 0, status: DownloadStatus::Failed,
                }).await?;
                return Err(anyhow::anyhow!("Failed to parse m3u8 playlist"));
            }
        };

        progress_tx.send(ProgressUpdate {
            video_id, downloaded: 0, total: playlist.segments.len() as u64,
            speed: 0.0, remaining_secs: 0, status: DownloadStatus::Downloading,
        }).await?;

        let mut file = tokio::fs::File::create(output_path).await?;
        let mut downloaded_segments = 0u64;
        let total_segments = playlist.segments.len() as u64;

        for (idx, segment) in playlist.segments.iter().enumerate() {
            if cancelled.load(Ordering::SeqCst) || self.global_cancelled.load(Ordering::SeqCst) {
                progress_tx.send(ProgressUpdate {
                    video_id, downloaded: 0, total: 0, speed: 0.0,
                    remaining_secs: 0, status: DownloadStatus::Cancelled,
                }).await?;
                return Err(anyhow::anyhow!("Download cancelled"));
            }

            let segment_filename = format!("{}_seg_{}.ts", output_path, idx);
            let mut segment_data: Vec<u8> = Vec::new();

            if PathBuf::from(&segment_filename).exists() {
                if let Ok(metadata) = tokio::fs::metadata(&segment_filename).await {
                    let _ = metadata.len(); // suppress unused result warning
                    if let Ok(data) = tokio::fs::read(&segment_filename).await {
                        segment_data = data;
                    }
                }
            }

            if segment_data.is_empty() {
                let seg_resp = client.get(&segment.uri).send().await?;
                let mut seg_stream = seg_resp.bytes_stream();
                while let Some(chunk) = seg_stream.next().await {
                    if cancelled.load(Ordering::SeqCst) || self.global_cancelled.load(Ordering::SeqCst) {
                        progress_tx.send(ProgressUpdate {
                            video_id, downloaded: 0, total: 0, speed: 0.0,
                            remaining_secs: 0, status: DownloadStatus::Cancelled,
                        }).await?;
                        return Err(anyhow::anyhow!("Download cancelled"));
                    }
                    let chunk = chunk?;
                    file.write_all(&chunk).await?;
                    segment_data.extend_from_slice(&chunk);
                }
                tokio::fs::write(&segment_filename, &segment_data).await?;
            }

            downloaded_segments += 1;
            let remaining = if downloaded_segments > 0 && downloaded_segments < total_segments {
                ((total_segments - downloaded_segments) as f64 / downloaded_segments as f64) as u64
            } else {
                0
            };

            progress_tx.send(ProgressUpdate {
                video_id, downloaded: downloaded_segments,
                total: total_segments, speed: 0.0,
                remaining_secs: remaining, status: DownloadStatus::Downloading,
            }).await?;
        }

        // Clean up segment files
        for i in 0..playlist.segments.len() {
            let seg_path = format!("{}_seg_{}.ts", output_path, i);
            let _ = tokio::fs::remove_file(&seg_path).await;
        }

        file.flush().await?;
        progress_tx.send(ProgressUpdate {
            video_id, downloaded: 1, total: 1, speed: 0.0,
            remaining_secs: 0, status: DownloadStatus::Completed,
        }).await?;
        Ok(())
    }

    pub async fn start_download(
        &self,
        video_id: u32,
        url: String,
        output_path: String,
        progress_tx: mpsc::Sender<ProgressUpdate>,
    ) -> Result<()> {
        {
            let mut tasks = self.tasks.lock().await;
            tasks.insert(video_id, DownloadTask {
                video_id, cancelled: Arc::new(AtomicBool::new(false)),
            });
        }
        {
            let mut count = self.active_count.lock().await;
            *count = count.saturating_add(1);
        }

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()?;

        progress_tx.send(ProgressUpdate {
            video_id, downloaded: 0, total: 0, speed: 0.0,
            remaining_secs: 0, status: DownloadStatus::Resolving,
        }).await?;

        if url.ends_with(".m3u8") {
            return self.download_m3u8(video_id, &url, &output_path, &progress_tx, &client, &Arc::new(AtomicBool::new(false))).await;
        }

        let mut resume_offset: u64 = 0;
        if PathBuf::from(&output_path).exists() {
            if let Ok(metadata) = tokio::fs::metadata(&output_path).await {
                resume_offset = metadata.len();
            }
        }

        let mut head_req = client.head(&url);
        if resume_offset > 0 {
            head_req = head_req.header("Range", format!("bytes={}-", resume_offset));
        }
        let resp = head_req.send().await?;
        let total_size = resp.content_length().unwrap_or(0);
        let effective_total = if resp.status() == reqwest::StatusCode::PARTIAL_CONTENT {
            resume_offset + total_size
        } else {
            total_size
        };

        if effective_total == 0 && resume_offset == 0 {
            progress_tx.send(ProgressUpdate {
                video_id, downloaded: 0, total: 0, speed: 0.0,
                remaining_secs: 0, status: DownloadStatus::Failed,
            }).await?;
            return Err(anyhow::anyhow!("Could not determine file size"));
        }

        progress_tx.send(ProgressUpdate {
            video_id, downloaded: resume_offset, total: effective_total,
            speed: 0.0, remaining_secs: 0, status: DownloadStatus::Downloading,
        }).await?;

        let mut file = if resume_offset > 0 {
            tokio::fs::OpenOptions::new().create(true).append(true).open(&output_path).await?
        } else {
            tokio::fs::File::create(&output_path).await?
        };

        let mut req = client.get(&url);
        if resume_offset > 0 {
            req = req.header("Range", format!("bytes={}-", resume_offset));
        }
        let resp = req.send().await?;
        let stream = resp.bytes_stream();
        let mut downloaded: u64 = resume_offset;
        let mut last_speed_time = std::time::Instant::now();
        let mut last_downloaded = resume_offset;

        let mut stream = stream.take(self.max_threads);
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    let bytes_len = bytes.len() as u64;
                    file.write_all(&bytes).await?;
                    downloaded += bytes_len;

                    let now = std::time::Instant::now();
                    if now.duration_since(last_speed_time).as_secs() >= 1 {
                        let elapsed = now.duration_since(last_speed_time).as_secs_f64();
                        let delta = downloaded - last_downloaded;
                        let speed = delta as f64 / elapsed;
                        let remaining = if effective_total > downloaded {
                            ((effective_total - downloaded) as f64 / speed) as u64
                        } else { 0 };
                        progress_tx.send(ProgressUpdate {
                            video_id, downloaded, total: effective_total,
                            speed, remaining_secs: remaining, status: DownloadStatus::Downloading,
                        }).await?;
                        last_speed_time = now;
                        last_downloaded = downloaded;
                    }
                }
                Err(e) => {
                    progress_tx.send(ProgressUpdate {
                        video_id, downloaded, total: effective_total,
                        speed: 0.0, remaining_secs: 0, status: DownloadStatus::Failed,
                    }).await?;
                    return Err(e.into());
                }
            }
        }

        file.flush().await?;
        progress_tx.send(ProgressUpdate {
            video_id, downloaded, total: effective_total,
            speed: 0.0, remaining_secs: 0, status: DownloadStatus::Completed,
        }).await?;
        Ok(())
    }

    pub async fn cancel_download(&self, video_id: u32) {
        let mut tasks = self.tasks.lock().await;
        if let Some(task) = tasks.get_mut(&video_id) {
            task.cancelled.store(true, Ordering::SeqCst);
        }
    }

    pub fn open_download_folder(path: &str) {
        if let Err(e) = open_folder(path) {
            eprintln!("Failed to open folder: {}", e);
        }
    }
}