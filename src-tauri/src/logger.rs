//! Logger module - handles log file persistence
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use anyhow::Result;

static LOG_FILE_PATH: &str = "VideoDownloader.log";
static MAX_LOG_ENTRIES: usize = 500;
static ENTRY_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Write a log entry to the log file
pub fn write_log(message: &str) -> Result<()> {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let log_line = format!("[{}] {}\n", timestamp, message);
    
    // Check if we need to rotate the log
    if ENTRY_COUNT.fetch_add(1, Ordering::SeqCst) >= MAX_LOG_ENTRIES {
        ENTRY_COUNT.store(0, Ordering::SeqCst);
        let _ = initialize_log_file();
    }
    
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE_PATH)?;
    
    file.write_all(log_line.as_bytes())?;
    Ok(())
}

/// Initialize or clear the log file
pub fn initialize_log_file() -> Result<()> {
    let _ = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(LOG_FILE_PATH)?;
    ENTRY_COUNT.store(0, Ordering::SeqCst);
    Ok(())
}

/// Read all log entries from the log file
pub fn read_log() -> Result<Vec<String>> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    
    let file = File::open(LOG_FILE_PATH)?;
    let reader = BufReader::new(file);
    let entries: Vec<String> = reader
        .lines()
        .filter_map(|line| line.ok())
        .collect();
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_and_read_log() {
        let test_message = "Test log message";
        write_log(test_message).expect("Failed to write log");
        let entries = read_log().expect("Failed to read log");
        assert!(entries.iter().any(|e| e.contains(test_message)));
    }
}
