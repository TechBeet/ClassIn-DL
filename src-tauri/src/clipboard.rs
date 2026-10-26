//! Clipboard module - handles reading, writing and background monitoring of the system clipboard.
use anyhow::Result;
use arboard::Clipboard;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::Notify;

/// Event emitted when the clipboard content changes and contains a ClassIn video URL.
pub const CLIPBOARD_DETECTED_EVENT: &str = "clipboard-detected";

/// How often the background watcher polls the clipboard.
const WATCH_INTERVAL_MS: u64 = 500;

/// Maximum clipboard length considered a candidate for parsing.
const MAX_CLIPBOARD_LEN: usize = 2 * 1024 * 1024;

/// Cancellation token for the clipboard watcher.
///
/// The watcher loop polls [`is_cancelled`] every cycle and additionally waits on the
/// [`Notify`] so that a stop request wakes it up immediately instead of waiting for
/// the next 500 ms tick.
#[derive(Clone, Default)]
pub struct ClipboardWatcher {
    cancelled: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl ClipboardWatcher {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Returns true when the watcher has been asked to stop.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Request the watcher to stop. Wakes up a sleeping watcher immediately.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    /// Reset the cancellation flag so the watcher can be started again.
    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
    }

    /// Wait until cancelled or until the timeout elapses. Returns true when cancelled.
    /// Note: Currently unused; kept for potential future use.
    #[allow(dead_code)]
    async fn wait_or_cancel(&self, timeout: Duration) -> bool {
        if self.is_cancelled() {
            return true;
        }
        // `notified()` must be created before the timeout to avoid a race where the
        // cancel signal arrives between the check and the await.
        let notified = self.notify.notified();
        tokio::select! {
            _ = notified => true,
            _ = tokio::time::sleep(timeout) => self.is_cancelled(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clipboard_write_read() {
        let text = "test clipboard content";
        write_clipboard(text).await.expect("Failed to write clipboard");
        let result = read_clipboard().await.expect("Failed to read clipboard");
        assert_eq!(result, text);
    }

    #[test]
    fn test_looks_like_video_content() {
        assert!(looks_like_video_content("http://a.com/x.mp4"));
        assert!(looks_like_video_content("http://a.com/x.m3u8?token=1"));
        assert!(looks_like_video_content("{\"url\":\"http://a.com/v\"}"));
        assert!(!looks_like_video_content("just some text"));
        assert!(!looks_like_video_content(""));
    }
}

/// Read clipboard text content inside a blocking task.
pub async fn read_clipboard() -> Result<String> {
    tokio::task::spawn_blocking(|| {
        let mut cb = Clipboard::new()?;
        let text = cb.get_text()?;
        Ok(text)
    }).await?
}

/// Write text to clipboard inside a blocking task.
pub async fn write_clipboard(text: &str) -> Result<()> {
    let owned_text = text.to_string();
    tokio::task::spawn_blocking(move || {
        let mut cb = Clipboard::new()?;
        cb.set_text(owned_text)?;
        Ok(())
    }).await?
}

/// Copy getLessonRecordInfo command to clipboard.
pub async fn copy_get_lesson_record_info_command() -> Result<()> {
    write_clipboard("getLessonRecordInfo").await?;
    Ok(())
}

/// Read clipboard text in a blocking STA-like thread (no async inside).
fn read_clipboard_blocking() -> Result<String> {
    let mut cb = Clipboard::new()?;
    let text = cb.get_text()?;
    Ok(text)
}

/// Heuristic check whether the clipboard text may contain a downloadable video.
pub fn looks_like_video_content(text: &str) -> bool {
    if text.contains(".m3u8") || text.contains(".mp4") || text.contains(".ts") {
        return true;
    }
    // JSON payloads from ClassIn carry URL fields even when the extension differs.
    if text.contains('{') && text.contains("url") {
        return true;
    }
    false
}

/// Start the background clipboard watcher.
///
/// The watcher runs on a dedicated Tokio blocking task and emits
/// `clipboard-detected` events whenever the clipboard changes and looks
/// like it contains a ClassIn video URL.
pub fn start_clipboard_watcher(app: AppHandle) -> ClipboardWatcher {
    let watcher = ClipboardWatcher::new();
    let watcher_clone = watcher.clone();

    tauri::async_runtime::spawn(async move {
        log::info!("Clipboard watcher started (interval {}ms)", WATCH_INTERVAL_MS);

        // Initialise "last seen" so we don't fire on startup.
        let last_text = Arc::new(std::sync::Mutex::new(String::new()));
        if let Ok(txt) = read_clipboard_blocking() {
            *last_text.lock().unwrap() = txt;
        }

        loop {
            // Sleep in small slices so cancellation is responsive.
            let deadline = tokio::time::Instant::now() + Duration::from_millis(WATCH_INTERVAL_MS);
            while tokio::time::Instant::now() < deadline {
                if watcher_clone.is_cancelled() {
                    log::info!("Clipboard watcher cancelled");
                    return;
                }
                // Wait for either the interval or an explicit cancel.
                let notified = watcher_clone.notify.notified();
                tokio::select! {
                    _ = notified => {
                        if watcher_clone.is_cancelled() { return; }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(50)) => {}
                }
            }

            if watcher_clone.is_cancelled() {
                log::info!("Clipboard watcher cancelled");
                return;
            }

            // Read the clipboard on a blocking thread (STA equivalent).
            let current_text = match tokio::task::spawn_blocking(read_clipboard_blocking).await {
                Ok(Ok(t)) => t,
                Ok(Err(e)) => {
                    log::debug!("Clipboard read failed: {e}");
                    continue;
                }
                Err(e) => {
                    log::error!("Clipboard blocking task failed: {e}");
                    continue;
                }
            };

            let mut last = last_text.lock().unwrap();
            if current_text == *last {
                continue;
            }
            *last = current_text.clone();

            if current_text.trim().is_empty() || current_text.len() > MAX_CLIPBOARD_LEN {
                continue;
            }
            if !looks_like_video_content(&current_text) {
                continue;
            }

            log::info!("Clipboard change detected ({} chars)", current_text.len());
            if let Err(e) = app.emit(CLIPBOARD_DETECTED_EVENT, current_text.clone()) {
                log::error!("Failed to emit clipboard event: {e}");
            }
        }
    });

    watcher
}