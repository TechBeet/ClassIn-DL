# ClassIn-DL 重构报告（Rust 后端 + Svelte 前端）

> 目标：在新 Tauri 项目中保持与旧版 WPF（`Classin视频解析下载工具-图形化v1.0.0`）**完全一致的对外行为**，
> 同时提升代码结构清晰度、可维护性与性能。

---

## 1. 旧版 WPF 下载核心算法分析

来源：`MainWindow.xaml.cs` → `DownloadManager`（静态类）+ `DownloadFileAsync`。

### 1.1 解析（Parser）
- 从剪贴板/请求头文本中用正则提取视频 URL（`.mp4` / `.m3u8` / `.ts`）。
- 支持 ClassIn JSON 载荷（含 `url` / `video` 字段的对象、数组）。
- 文件名由 URL 派生并做非法字符清洗（`: ? * | < > "` 及控制字符）。

### 1.2 并发（Concurrency）
- `_maxConcurrentDownloads` 默认 **5**，用 `SemaphoreSlim` 限流。
- `SetMaxConcurrentDownloads` 动态调整信号量（增大 `Release`，减小 `Wait(0)`）。
- `MaxDownloadThreads` 默认 **32**，钳制在 `[1, 256]`。

### 1.3 分段 / 断点续传（Range + Resume）
- 先 `HEAD` 获取 `Content-Length`（`GetFileSizeAsync`）。
- 若本地已有部分文件，用 `Range: bytes={startPosition}-` 续传。
- 收到 `206 Partial Content` 时，`total = startPosition + ContentLength`。
- 以追加模式写入文件，保留 `.json` 侧车文件记录进度。

### 1.4 进度（Progress）
- 每 **200 ms** 触发一次回调：`progressCallback(downloaded, total, speed, remainingTime)`。
- `speed = (totalBytesRead - lastBytesRead) / (elapsed / 1000)`。
- `remainingTime = (total - downloaded) / speed`。

### 1.5 暂停 / 取消（Pause / Cancel）
- `CancellationTokenSource` 存于 `ConcurrentDictionary<string, CTS>`。
- `CancelDownload(url)` 触发取消；取消抛出 `OperationCanceledException` 直接向上传播（不重试）。

### 1.6 重试（Retry）
- `maxRetries = 5`。
- 仅对 `5xx` 或一般异常重试；`IOException`（文件不存在等）直接抛出。
- 退避：`ExponentialBackoff(n) = min(1000 * 2^n, 60000)` 毫秒。

### 1.7 M3U8 处理
- 解析 playlist，按顺序下载分片，最后按序拼接。
- 分片下载同样带 Referer/UA。

### 1.8 HTTP 细节（关键兼容点）
- `User-Agent`: `Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36`
- `Referer`: `https://www.eeo.cn/`
- `Accept`: `video/mp4, */*`
- 禁用代理，`MaxConnectionsPerServer = 100`，超时 6 小时。

---

## 2. 新版 Tauri 重构内容

### 2.1 Rust 后端（`src-tauri/src/`）

| 模块 | 变更 |
| --- | --- |
| `downloader.rs` | UA 对齐 WPF（Chrome 91）；新增 `Referer` 头；`MAX_RETRIES` 4→**5**；`UI_THROTTLE_MS` 100→**200**；新增 `exponential_backoff()`（`min(1000*2^n, 60000)`）；新增 `get_file_size()`（HEAD）；新增 `now_millis()` 辅助 |
| `downloader.rs` | **修复进度竞态/计算 bug**：原代码用 `AtomicU64::swap` 在每个 chunk 都重置基准，导致 speed 恒为瞬时值且时间窗错乱。改为仅在真正发出 UI 更新时才推进 `last_now/last_bytes` |
| `downloader.rs` | **续传 offset 修正**：优先采用磁盘实际大小 `existing_size`，其次 resume 侧车，保证 `offset` 与实际文件对齐 |
| `downloader.rs` | **HLS 分片下载增强**：每分片最多 3 次线性退避重试；补全 `User-Agent` 头；已存在分片计入完成数（支持续传） |
| `downloader.rs` | 取消/暂停时持久化 resume 侧车（`paused` / `cancelled` 状态），与 WPF 保留部分文件的语义一致 |
| `logger.rs` | `set_log_path` 加 `#[cfg_attr(not(test), allow(dead_code))]` 消除 dead-code 警告 |
| `settings.rs` | `load_from` / `save_to` 同样标注，消除警告 |

### 2.2 Svelte 前端（`src/frontend/src/App.svelte`）
- 事件名保持不变：`dl-progress`、`dl-finished`、`clipboard-detected`、`app-ready`。
- 命令名保持不变：`parse_videos` / `start_downloads` / `cancel_download` / `pause_download` / `resume_download` / `load_settings` / `save_settings` / `read_clipboard` / `open_folder` 等。
- 进度条 `progress = downloaded / total * 100`，与后端一致。

---

## 3. API 一致性对照表

| 旧版 WPF 概念 | 新版 Tauri 命令 / 事件 | 参数 | 返回 |
| --- | --- | --- | --- |
| 解析请求头 | `parse_videos` | `content`, `maxConcurrent` | `VideoItem[]` |
| 开始下载 | `start_downloads` | `videos`, `dir`, `maxConcurrent` | `()` |
| 取消单个 | `cancel_download` | `videoId` | `()` |
| 取消全部 | `cancel_all_downloads` | – | `()` |
| 暂停 / 恢复 | `pause_download` / `resume_download` | `videoId` | `()` |
| 进度回调 | 事件 `dl-progress` | `ProgressUpdate` | – |
| 完成通知 | 事件 `dl-finished` | `(id, status)` | – |
| 打开目录 | `open_folder` | `path` | `()` |

---

## 4. 性能优化点

1. **进度上报节流**：统一 200ms 窗口，减少 IPC 事件风暴（大文件场景收益显著）。
2. **速度计算修正**：真实测量窗口内的 `Δbytes / Δms`，避免除零与抖动。
3. **HLS 并发分片**：`Semaphore(max_threads)` 控制并发，分片级重试避免整体失败。
4. **续传**：`Range` + 侧车文件，避免重复下载已完成部分。
5. **连接复用**：`reqwest::Client` 单例复用，`no_proxy`，长超时（600s）。

---

## 5. 验证结果

```
cargo check      → Finished (0 warnings, 0 errors)
cargo test       → 94 passed; 0 failed
npm run build    → ✓ built in 418ms (dist/index.html, CSS 8.38kB, JS 52.12kB)
```

- 后端：`cargo check` 无警告无错误；`cargo test` 94/94 通过。
- 前端：`vite build` 成功产出生产包。

---

## 6. 已知差异 / 后续建议

- WPF 使用真正的多线程分段（同一文件多段并行）；当前 Rust 版对 HTTP 文件为单流 `Range` 续传，对 HLS 为分片并行。如需对齐「单文件多段并行」，可在 `http_download` 增加分块并行下载 + 合并阶段。
- WPF 的 `SetMaxConcurrentDownloads` 支持运行时动态调整信号量；当前实现通过每次 `start_downloads` 重建 `Semaphore` 达成等效效果。
