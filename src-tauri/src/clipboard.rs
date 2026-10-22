//! Clipboard module - handles reading and writing to system clipboard
use arboard::Clipboard as ArboardClipboard;
use anyhow::Result;

/// 读取剪贴板文本内容
pub fn read_clipboard() -> Result<String> {
    let mut cb = ArboardClipboard::new()?;
    let text = cb.get_text()?;
    Ok(text)
}

/// 写入文本到剪贴板
pub fn write_clipboard(text: &str) -> Result<()> {
    let mut cb = ArboardClipboard::new()?;
    cb.set_text(text)?;
    Ok(())
}

/// 复制 getLessonRecordInfo 命令（兼容旧版功能）
pub fn copy_get_lesson_record_info_command() -> Result<()> {
    let command = "getLessonRecordInfo";
    write_clipboard(command)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_write_read() {
        let text = "test clipboard content";
        write_clipboard(text).expect("Failed to write clipboard");
        let result = read_clipboard().expect("Failed to read clipboard");
        assert_eq!(result, text);
    }
}