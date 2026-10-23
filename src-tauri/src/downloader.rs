//! Downloader module - handles video download operations with resume support.
use crate::models::{DownloadStatus, DownloadResume, ProgressUpdate, VideoItem};
use crate::m3u8;
use anyhow::{Context, Result};
use futures_util::stream::StreamExt;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, Semaphore};

const UA: &str = concat!("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36");
const REFERER: &str = "https://www.eeo.cn/";
const MAX_RETRIES: u32 = 4;
const UI_THROTTLE_MS: u64 = 100;
const CHUNK: usize = 65536;
pub const EVT_PROGRESS: &str = "dl-progress";
pub const EVT_FINISHED: &str = "dl-finished";

fn emit(app: &AppHandle, u: ProgressUpdate) {
    if let Err(e) = app.emit(EVT_PROGRESS, u.clone()) {
        log::warn!("emit progress: {e}");
    }
}

fn part_path(p: &Path) -> PathBuf { p.with_extension("part") }
fn resume_path(p: &Path) -> PathBuf { p.with_extension("json") }

fn build_client() -> Result<reqwest::Client> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(reqwest::header::REFERER, REFERER.parse().context("referer header")?);
    reqwest::Client::builder()
        .user_agent(UA)
        .default_headers(headers)
        .timeout(Duration::from_secs(600))
        .connect_timeout(Duration::from_secs(15))
        .no_proxy()
        .build()
        .context("reqwest client")
}

fn load_resume(p: &Path) -> Option<DownloadResume> {
    let rp = resume_path(p);
    if let Ok(data) = std::fs::read_to_string(&rp) {
        serde_json::from_str(&data).ok()
    } else { None }
}

fn save_resume(p: &Path, r: &DownloadResume) -> Result<()> {
    let rp = resume_path(p);
    let data = serde_json::to_string_pretty(r)?;
    std::fs::write(&rp, data)?;
    Ok(())
}

#[derive(Clone)]
struct TaskControl { cancelled: Arc<AtomicBool>, paused: Arc<AtomicBool> }

#[derive(Clone)]
pub struct DownloadManager {
    max_concurrent: usize,
    max_threads: usize,
    tasks: Arc<Mutex<HashMap<u32, TaskControl>>>,
    global_cancel: Arc<AtomicBool>,
}

impl DownloadManager {
    pub fn new(max_concurrent: usize, max_threads: usize) -> Self {
        Self { max_concurrent, max_threads, tasks: Default::default(), global_cancel: Default::default() }
    }

    pub async fn start_downloads(
        &self,
        app: AppHandle,
        videos: Vec<VideoItem>,
        dir: String,
        max_concurrent: usize,
    ) -> Result<()> {
        self.global_cancel.store(false, Ordering::SeqCst);
        tokio::fs::create_dir_all(&dir).await?;
        let sem = Arc::new(Semaphore::new(max_concurrent.max(1)));
        let client = build_client()?;
        let dir = PathBuf::from(dir);

        for video in videos {
            let permit = sem.clone().acquire_owned().await.context("semaphore")?;
            let ctrl = TaskControl { cancelled: Arc::new(AtomicBool::new(false)), paused: Arc::new(AtomicBool::new(false)) };
            {
                let mut lock = self.tasks.lock().await;
                lock.insert(video.id, ctrl.clone());
            }
            let app2 = app.clone();
            let dir2 = dir.clone();
            let cli = client.clone();
            let gc = self.global_cancel.clone();
            let tasks = self.tasks.clone();
            let threads = self.max_threads;
            tokio::spawn(async move {
                let _p = permit;
                let is_hls = m3u8::is_m3u8_url(&video.url);
                let out = crate::models::resolve_video_path(dir2.to_string_lossy().as_ref(), &video, is_hls);
                let res = download_one(&app2, &cli, &video, &PathBuf::from(&out), threads, &ctrl, &gc).await;
                match &res {
                    Ok(_) => log::info!("Done: {}", out),
                    Err(e) => log::error!("Failed {}: {e}", video.id),
                }
                {
                    let mut lock = tasks.lock().await;
                    lock.remove(&video.id);
                }
                let status = match &res { Ok(_) => DownloadStatus::Completed, Err(_) => DownloadStatus::Failed };
                emit(&app2, ProgressUpdate::new(video.id, status.clone(), "已完成"));
                let _ = app2.emit(EVT_FINISHED, (video.id, status));
            });
        }
        Ok(())
    }

    pub async fn cancel_download(&self, video_id: u32) {
        if let Some(t) = self.tasks.lock().await.remove(&video_id) {
            t.cancelled.store(true, Ordering::SeqCst);
        }
    }

    pub async fn cancel_all(&self) {
        self.global_cancel.store(true, Ordering::SeqCst);
        let tasks = self.tasks.lock().await;
        for (_, t) in tasks.iter() {
            t.cancelled.store(true, Ordering::SeqCst);
        }
    }

    pub async fn set_paused(&self, video_id: u32, paused: bool) {
        if let Some(t) = self.tasks.lock().await.get_mut(&video_id) {
            t.paused.store(paused, Ordering::SeqCst);
        }
    }

    pub fn open_download_folder(path: &str) { let _ = open::that(path); }
}

async fn download_one(
    app: &AppHandle,
    client: &reqwest::Client,
    video: &VideoItem,
    out_path: &Path,
    max_threads: usize,
    ctrl: &TaskControl,
    global: &Arc<AtomicBool>,
) -> Result<()> {
    let is_hls = m3u8::is_m3u8_url(&video.url);
    emit(app, ProgressUpdate::new(video.id, DownloadStatus::Resolving, "解析中"));
    if is_hls { hls_download(app, client, video, out_path, max_threads, ctrl, global).await }
    else { http_download(app, client, video, out_path, ctrl, global).await }
}

async fn http_download(
    app: &AppHandle,
    client: &reqwest::Client,
    video: &VideoItem,
    out_path: &Path,
    ctrl: &TaskControl,
    global: &Arc<AtomicBool>,
) -> Result<()> {
    let url = &video.url;
    let mut file = if out_path.exists() {
        File::open(out_path).await?
    } else {
        File::create(out_path).await?
    };
    let resume: DownloadResume = load_resume(out_path).unwrap_or_default();
    let offset: u64 = if resume.file_size > resume.downloaded && resume.downloaded > 0 {
        resume.downloaded
    } else { 0u64 };
    let resp = client.head(url).send().await?;
    let total: u64 = if offset == 0 {
        resp.content_length().unwrap_or(0)
    } else {
        let r: DownloadResume = serde_json::from_str(&serde_json::to_string(&resume).unwrap()).unwrap_or_default();
        r.file_size
    };
    emit(app, ProgressUpdate::new(video.id, DownloadStatus::Downloading, "下载中"));
    let url = url.clone();
    for attempt in 1..=MAX_RETRIES {
        if global.load(Ordering::SeqCst) || ctrl.cancelled.load(Ordering::SeqCst) { return Err(anyhow::anyhow!("cancelled")); }
        if attempt > 1 { tokio::time::sleep(Duration::from_secs(1 << (attempt - 2))).await; }
        let req = client.get(&url).header("Range", format!("bytes={}-", offset));
        match req.send().await {
            Ok(resp) => {
                if resp.status().is_success() || (resp.status() == 206) {
                    let mut stream = resp.bytes_stream();
                    let downloaded = AtomicU64::new(offset);
                    let last_now = AtomicU64::new(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64);
                    let last_bytes = AtomicU64::new(offset);
                    while let Some(chunk) = stream.next().await {
                        if global.load(Ordering::SeqCst) || ctrl.cancelled.load(Ordering::SeqCst) {
                            return Err(anyhow::anyhow!("cancelled"));
                        }
                        let bytes = chunk?;
                        file.write_all(&bytes).await?;
                        let cur = downloaded.fetch_add(bytes.len() as u64, Ordering::SeqCst);
                        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
                        let prev = last_now.swap(now, Ordering::SeqCst);
                        let prev_bytes = last_bytes.swap(cur + bytes.len() as u64, Ordering::SeqCst);
                        if now.saturating_sub(prev) >= UI_THROTTLE_MS {
                            let dl = cur + bytes.len() as u64;
                            let speed = if prev > 0 { ((dl - prev_bytes) as f64 / (now - prev) as f64) * 1000.0 } else { 0.0 };
                            let _remain = if total > 0 && dl < total { ((total - dl) as f64 / speed).ceil() as u64 } else { 0u64 };
                            let _pct = if total > 0 { dl as f64 / total as f64 * 100.0 } else { 0.0 };
                            emit(app, ProgressUpdate {
                                video_id: video.id,
                                downloaded: dl,
                                total,
                                speed,
                                remaining_secs: _remain,
                                status: DownloadStatus::Downloading,
                                phase: "下载中".into(),
                                speed_total: speed,
                                completed_count: 0,
                            });
                        }
                    }
                    file.flush().await?;
                    save_resume(out_path, &DownloadResume { video_id: video.id, url: url.clone(), file_size: total, downloaded: offset, segments: vec![], status: "done".into() }).ok();
                    emit(app, ProgressUpdate::new(video.id, DownloadStatus::Completed, "已完成"));
                    return Ok(());
                } else {
                    log::warn!("HTTP status {} for {}", resp.status(), url);
                }
            }
            Err(e) => { log::debug!("Retry {} error: {e}", attempt); }
        }
    }
    Err(anyhow::anyhow!("max retries reached"))
}

async fn hls_download(
    app: &AppHandle,
    client: &reqwest::Client,
    video: &VideoItem,
    out_path: &Path,
    max_threads: usize,
    ctrl: &TaskControl,
    global: &Arc<AtomicBool>,
) -> Result<()> {
    let url = &video.url;
    emit(app, ProgressUpdate::new(video.id, DownloadStatus::Resolving, "解析M3U8"));
    let playlist_text = client.get(url).header("Referer", REFERER).send().await?.text().await?;
    let base = url.strip_suffix(url.split('/').last().unwrap_or("")).unwrap_or(url);
    let playlist = m3u8::M3u8Playlist::parse(&playlist_text, base).context("parse m3u8")?;
    let segments = playlist.segments;
    let temp_dir = out_path.with_extension("parts");
    tokio::fs::create_dir_all(&temp_dir).await?;
    let sem = Arc::new(Semaphore::new(max_threads.max(1)));
    let total_segs = segments.len();
    let done = Arc::new(AtomicU64::new(0));
    for (i, seg) in segments.iter().enumerate() {
        if global.load(Ordering::SeqCst) || ctrl.cancelled.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("cancelled"));
        }
        let p = temp_dir.join(format!("{i:04}.ts"));
        let p_exists = p.exists();
        if !p_exists {
            let uri = seg.uri.clone();
            let cli = client.clone();
            tokio::spawn(async move {
                let result = match cli.get(&uri).header("Referer", REFERER).send().await {
                    Ok(r) => r.bytes().await.ok(),
                    Err(_) => None,
                };
                if let Some(bytes) = result {
                    std::fs::write(&p, &bytes).ok();
                }
            });
            {
                let _permit = sem.clone().acquire_owned().await.context("semaphore")?;
                // permit is automatically dropped when out of scope
            }
        }
    }
    // Wait for all segments
    loop {
        if global.load(Ordering::SeqCst) || ctrl.cancelled.load(Ordering::SeqCst) { return Err(anyhow::anyhow!("cancelled")); }
        tokio::time::sleep(Duration::from_millis(100)).await;
        let cur = done.load(Ordering::SeqCst);
        let _ = cur;
        let f: u64 = match std::fs::read_dir(&temp_dir) {
            Ok(rd) => rd.flatten().filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("ts")).count() as u64,
            Err(_) => continue,
        };
        done.store(f, Ordering::SeqCst);
        if f >= total_segs as u64 { break; }
        let pct = f as f64 / total_segs as f64 * 100.0;
        emit(app, ProgressUpdate { video_id: video.id, downloaded: f * 1024, total: total_segs as u64 * 1024, speed: pct, remaining_secs: 0, status: DownloadStatus::Downloading, phase: format!("分段下载 {}/{}", f, total_segs), speed_total: 0.0, completed_count: 0 });
    }
    // Concatenate
    emit(app, ProgressUpdate::new(video.id, DownloadStatus::Downloading, "合并"));
    let mut out = File::create(out_path).await?;
    for i in 0..total_segs {
        let p = temp_dir.join(format!("{i:04}.ts"));
        if p.exists() {
            let data = tokio::fs::read(&p).await?;
            out.write_all(&data).await?;
        }
    }
    out.flush().await?;
    // Clean up
    let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    emit(app, ProgressUpdate::new(video.id, DownloadStatus::Completed, "已完成"));
    Ok(())
}