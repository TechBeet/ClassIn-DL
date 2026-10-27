//! Logger module - handles log file persistence
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{OnceLock, RwLock};
use anyhow::Result;

const DEFAULT_LOG_FILE: &str = "VideoDownloader.log";
static MAX_LOG_ENTRIES: usize = 500;
static ENTRY_COUNT: AtomicUsize = AtomicUsize::new(0);
static LOG_PATH: OnceLock<RwLock<PathBuf>> = OnceLock::new();

fn log_path_slot() -> &'static RwLock<PathBuf> {
    LOG_PATH.get_or_init(|| RwLock::new(PathBuf::from(DEFAULT_LOG_FILE)))
}

/// Resolve the active log file path.
fn log_path() -> PathBuf {
    log_path_slot().read().map(|p| p.clone()).unwrap_or_else(|_| {
        PathBuf::from(DEFAULT_LOG_FILE)
    })
}

/// Override the log file path. Intended for tests and advanced configuration.
pub fn set_log_path<P: Into<PathBuf>>(path: P) {
    if let Ok(mut w) = log_path_slot().write() {
        *w = path.into();
    }
}

/// Write a log entry to the log file
#[allow(dead_code)]
pub fn write_log(message: &str) -> Result<()> {
    write_log_to(&log_path(), message)
}

/// Write a log entry to an explicit log file path.
fn write_log_to(path: &Path, message: &str) -> Result<()> {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let log_line = format!("[{}] {}\n", timestamp, message);

    // Rotate the log once it grows past the maximum number of entries.
    if ENTRY_COUNT.fetch_add(1, Ordering::SeqCst) >= MAX_LOG_ENTRIES {
        ENTRY_COUNT.store(0, Ordering::SeqCst);
        let _ = initialize_log_file();
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    file.write_all(log_line.as_bytes())?;
    Ok(())
}

/// Initialize or clear the log file
pub fn initialize_log_file() -> Result<()> {
    initialize_log_file_at(&log_path())
}

/// Initialize or clear the log file at an explicit path.
fn initialize_log_file_at(path: &Path) -> Result<()> {
    let _ = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;
    ENTRY_COUNT.store(0, Ordering::SeqCst);
    Ok(())
}

/// Read all log entries from the log file
pub fn read_log() -> Result<Vec<String>> {
    read_log_from(&log_path())
}

/// Read all log entries from an explicit log file path.
fn read_log_from(path: &Path) -> Result<Vec<String>> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let entries: Vec<String> = reader
        .lines()
        .map_while(std::result::Result::ok)
        .collect();
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Simple serial test runner for logger tests to avoid races
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    /// Each test takes a unique log file and runs serially.
    fn use_temp_log() -> PathBuf {
        let _guard = TEST_LOCK.lock().unwrap();
        let path = std::env::temp_dir().join(format!(
            "classin-dl-test-log-{}-{:?}.txt",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_file(&path);
        set_log_path(path.clone());
        let _ = initialize_log_file_at(&path);
        path
    }

    #[test]
    fn test_write_and_read_log() {
        let path = use_temp_log();
        let test_message = "Test log message";
        write_log_to(&path, test_message).expect("Failed to write log");
        let entries = read_log_from(&path).expect("Failed to read log");
        assert!(entries.iter().any(|e| e.contains(test_message)));
    }

    #[test]
    fn test_log_contains_timestamp() {
        let path = use_temp_log();
        write_log_to(&path, "timestamp test").expect("write failed");
        let entries = read_log_from(&path).expect("read failed");
        let last_entry = entries.last().unwrap();
        assert!(last_entry.contains("20") && last_entry.contains(":"));
    }

    #[test]
    fn test_initialize_log_clears_file() {
        let path = use_temp_log();
        let test_msg = "first message";
        write_log_to(&path, test_msg).expect("write failed");
        initialize_log_file_at(&path).expect("clear failed");
        let entries = read_log_from(&path).expect("read failed");
        assert!(!entries.iter().any(|e| e.contains(test_msg)));
    }

    #[test]
    fn test_multiple_writes_accumulate() {
        let path = use_temp_log();
        write_log_to(&path, "msg1").expect("write 1 failed");
        write_log_to(&path, "msg2").expect("write 2 failed");
        write_log_to(&path, "msg3").expect("write 3 failed");
        let entries = read_log_from(&path).expect("read failed");
        assert_eq!(entries.len(), 3);
        assert!(entries.iter().any(|e| e.contains("msg1")));
        assert!(entries.iter().any(|e| e.contains("msg2")));
        assert!(entries.iter().any(|e| e.contains("msg3")));
    }

    #[test]
    fn test_read_log_empty_file() {
        let path = use_temp_log();
        let entries = read_log_from(&path).expect("read failed");
        assert!(entries.is_empty());
    }

    #[test]
    fn test_log_format() {
        let path = use_temp_log();
        write_log_to(&path, "test format").expect("write failed");
        let entries = read_log_from(&path).expect("read failed");
        let line = &entries[0];
        assert!(line.starts_with('['));
        assert!(line.contains(']'));
        assert!(line.contains("test format"));
    }
}
