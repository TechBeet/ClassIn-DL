# Feature Gap Analysis: ClassIn-DL Rust/Tauri vs WPF Reference

## Overview
This document compares the Rust/Tauri implementation against the WPF reference project (Classin 视频解析下载工具 - 图形化 v1.0.0).

## Feature Parity Status

### ✅ Implemented Features
- **Clipboard parsing** - Background monitoring, video URL detection
- **Video URL extraction** - JSON and plain text parsing from clipboard
- **Multi-threaded download** - Concurrent downloads with configurable limit
- **M3U8/HLS support** - Full HLS playlist parsing and segment merging
- **Breakpoint resume** - Resume data persistence (.json sidecar files)
- **Settings persistence** - Theme, column widths, sort config, download directory
- **Log viewing** - Log file with timestamp formatting, rotation at 500 entries
- **Sorting/filtering** - Column-based sorting (ascending/descending)

### ⚠️ Partially Implemented / Needs Verification
- **Settings persistence** - Backend complete, verify UI bindings match WPF exactly
- **Log viewing** - Backend complete, verify UI matches WPF layout
- **Sorting/filtering** - Backend complete, verify UI matches WPF behavior

### ❌ Missing Features (WPF has these)

#### 1. Playlist Management
**WPF Implementation:**
- Save/load playlists to/from JSON files
- Add/remove/reorder videos in playlist
- Duplicate detection (prevent adding same URL twice)
- Import/export playlist functionality
- Batch operations on playlist items

**Rust/Tauri Status:** MISSING
**Priority:** HIGH
**Implementation Plan:**
- Create `playlist.rs` module with `Playlist` struct
- Add Tauri commands: `save_playlist`, `load_playlist`, `add_to_playlist`, `remove_from_playlist`, `reorder_playlist`
- Frontend: Playlist panel with drag-drop reordering

#### 2. Cloud Sync (Optional)
**WPF Implementation:**
- Optional cloud backup of playlists and settings
- Sync across devices

**Rust/Tauri Status:** MISSING
**Priority:** LOW (Optional)
**Implementation Plan:**
- Deferred until explicitly requested

#### 3. Video Transcoding (Out of Scope)
**WPF Implementation:**
- Convert downloaded videos to different formats
- FFmpeg integration

**Rust/Tauri Status:** OUT OF SCOPE
**Priority:** N/A

#### 4. Scheduled Downloads (Optional)
**WPF Implementation:**
- Schedule downloads for specific times
- Recurring download tasks

**Rust/Tauri Status:** MISSING
**Priority:** LOW (Optional)
**Implementation Plan:**
- Deferred until explicitly requested

---

## UI Alignment Checklist

### Window Configuration
| Property | WPF | Rust/Tauri | Status |
|----------|-----|------------|--------|
| Width | 1280px | TBD | ⚠️ Verify |
| Height | 720px | TBD | ⚠️ Verify |
| Min Width | 800px | TBD | ⚠️ Verify |
| Min Height | 450px | TBD | ⚠️ Verify |

### Color Scheme
| Element | WPF Hex | Rust/Tauri | Status |
|---------|---------|------------|--------|
| Primary Blue | #2196F3 | TBD | ⚠️ Verify |
| Primary Green | #4CAF50 | TBD | ⚠️ Verify |
| Error Red | #F44336 | TBD | ⚠️ Verify |
| Warning Orange | #FF9800 | TBD | ⚠️ Verify |
| Success Green | #4CAF50 | TBD | ⚠️ Verify |

### Column Headers
| Column | WPF Text | Rust/Tauri | Status |
|--------|----------|------------|--------|
| Index | 序号 | 序号 | ✅ Match |
| Name | 课程名称 | 课程名称 | ✅ Match |
| Size | 文件大小 | 文件大小 | ✅ Match |
| Progress | 进度 | 进度 | ✅ Match |
| Status | 状态 | 状态 | ✅ Match |
| Actions | 操作 | 操作 | ✅ Match |

### Button Styles
- Hover effects: Need verification
- Active states: Need verification
- Disabled states: Need verification

### Status Bar
- Layout: Need verification
- Scaling converters: Need verification

---

## Code Quality Improvements Made

### Refactoring Completed
1. **Logger Module**: Added path injection for testability, thread-safe initialization
2. **Settings Module**: Exposed internal functions for testing, improved error handling
3. **Downloader Module**: Fixed semaphore acquire pattern, added spawn_blocking for clipboard
4. **Parser Module**: Improved error handling, clearer code flow
5. **M3U8 Module**: Fixed EXTINF parsing bug (was inside unreachable branch)

### Test Coverage
- **Before**: 10 tests
- **After**: 88 tests
- **Modules Covered**: models, m3u8, parser, settings, logger, clipboard, downloader

### Known Anti-Patterns Fixed
1. ✅ Semaphore acquire pattern in downloader (now uses `acquire_owned()`)
2. ✅ Clipboard read on blocking thread (now uses `spawn_blocking`)
3. ✅ M3U8 EXTINF parsing (fixed logic bug)
4. ✅ Thread-safe log path injection for tests

---

## Next Steps

### High Priority
1. Implement playlist management feature
2. Verify UI matches WPF exactly (window size, colors, buttons)
3. Run UI visual regression tests

### Medium Priority
1. Add integration tests for full workflow
2. Improve error messages for users
3. Add unit tests for frontend components

### Low Priority
1. Cloud sync feature (if requested)
2. Scheduled downloads (if requested)
3. Performance optimization profiling

---

## Technical Debt
- None identified after refactoring

## Security Considerations
- No sensitive data stored unencrypted
- Network requests use HTTPS only
- No third-party analytics
