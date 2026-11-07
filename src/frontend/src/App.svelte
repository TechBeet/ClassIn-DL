<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';

  interface VideoItem {
    id: number; display_index: string; name: string; url: string;
    file_size: number; downloaded_bytes: number; status: string;
    progress: number; current_speed: number; remaining_time_secs: number;
    phase: string; error_message: string; selected?: boolean;
    status_color?: string;
  }
  interface ProgressUpdate {
    video_id: number; downloaded: number; total: number; speed: number;
    remaining_secs: number; status: string; phase: string;
    speed_total: number; completed_count: number;
  }
  interface ColumnWidths {
    idx: number; name: number; size: number; progress: number; status: number; action: number;
  }
  interface AppSettings {
    theme: string;
    column_widths: ColumnWidths;
    sort: { column: string; ascending: boolean };
    max_concurrent: number;
    download_dir: string;
  }

  const DEFAULT_WIDTHS: ColumnWidths = { idx: 60, name: 150, size: 120, progress: 250, status: 120, action: 150 };

  let videos = $state<VideoItem[]>([]);
  let statusText = $state('就绪');
  let downloadDir = $state('');
  let maxConcurrent = $state(5);
  let maxThreads = $state(32);
  let isParsing = $state(false);
  let isDownloading = $state(false);
  let errorBox = $state('');
  let logs = $state<string[]>([]);
  let inputText = $state('');
  let widths = $state<ColumnWidths>({ ...DEFAULT_WIDTHS });
  let clipboardWatch = $state(true);
  let parseProgress = $state(0);
  let overallSpeed = $state(0);
  let completedCount = $state(0);
  let lastSelectedIndex = $state(-1);
  let contextMenu = $state<{ x: number; y: number; video: VideoItem | null } | null>(null);
  let draggingCol = $state<string | null>(null);
  let dragStartX = $state(0);
  let dragStartW = $state(0);

  let unlisteners: UnlistenFn[] = [];
  let pollTimer: ReturnType<typeof setInterval> | undefined;

  const sortedVideos = $derived(videos);

  const totalCount = $derived(videos.length);
  const doneCount = $derived(videos.filter(v => isDone(v.status)).length);
  const failedCount = $derived(videos.filter(v => v.status.toLowerCase().includes('fail')).length);
  const activeCount = $derived(videos.filter(v => v.status.toLowerCase().includes('down')).length);
  const selectedCount = $derived(videos.filter(v => v.selected).length);

  function isDone(s: string) {
    const l = s.toLowerCase();
    return l.includes('done') || l.includes('complete') || l.includes('已完成');
  }

  function getStatusColor(status: string): string {
    const s = status.toLowerCase();
    if (s.includes('done') || s.includes('complete') || s.includes('已完成')) return '#4CAF50';
    if (s.includes('fail') || s.includes('error') || s.includes('失败')) return '#F44336';
    if (s.includes('cancel') || s.includes('取消')) return '#9E9E9E';
    if (s.includes('pause') || s.includes('暂停')) return '#FF9800';
    if (s.includes('partial') || s.includes('部分')) return '#00BCD4';
    if (s.includes('down') || s.includes('下载')) return '#2196F3';
    if (s.includes('resolv') || s.includes('解析')) return '#9C27B0';
    if (s.includes('wait') || s.includes('等待')) return '#FF9800';
    return '#607D8B';
  }

  onMount(async () => {
    try { await invoke('show_window'); } catch(e) { console.error(e); }
    await loadSettings();
    await initApp();
    unlisteners.push(await listen<ProgressUpdate>('dl-progress', (e) => applyProgress(e.payload)));
    unlisteners.push(await listen<string>('clipboard-detected', (e) => {
      if (clipboardWatch) { statusText = '检测到剪贴板内容，正在解析...'; void doParse(e.payload); }
    }));
    unlisteners.push(await listen('app-ready', async () => { statusText = '就绪'; try { await invoke('show_window'); } catch(e) { console.error(e); } }));
  });

  onDestroy(() => {
    unlisteners.forEach(u => u());
    if (pollTimer) clearInterval(pollTimer);
  });

  async function loadSettings() {
    try {
      const s = await invoke<AppSettings>('load_settings');
      if (s.column_widths && (s.column_widths.idx || 0) > 0) {
        widths = { ...DEFAULT_WIDTHS, ...s.column_widths } as ColumnWidths;
      }
      if (s.max_concurrent > 0) maxConcurrent = s.max_concurrent;
      if (s.download_dir) downloadDir = s.download_dir;
    } catch (e) { console.error(e); }
  }

  async function persistSettings() {
    const payload: AppSettings = {
      theme: 'light',
      column_widths: widths,
      sort: { column: '', ascending: true },
      max_concurrent: Number(maxConcurrent),
      download_dir: downloadDir,
    };
    try { await invoke('save_settings', { newSettings: payload }); }
    catch (e) { console.error(e); }
  }  async function initApp() {
    try { const dir = await invoke<string>('get_default_dir'); if (!downloadDir) downloadDir = dir; } catch (e) { console.error(e); }
    try { await invoke('initialize_log'); } catch (e) { console.error(e); }
    pollTimer = setInterval(async () => { try { logs = (await invoke<string[]>('read_logs')) || []; } catch (e) { /* ignore */ } }, 2000);
  }

  function applyProgress(u: ProgressUpdate) {
    const idx = videos.findIndex(v => v.id === u.video_id);
    overallSpeed = u.speed_total || 0;
    if (idx >= 0) {
      const v = videos[idx];
      v.downloaded_bytes = u.downloaded;
      if (u.total > 0) v.file_size = u.total;
      v.current_speed = u.speed;
      v.remaining_time_secs = u.remaining_secs;
      v.status = u.status;
      v.phase = u.phase;
      if (u.total > 0) v.progress = Math.min(100, (u.downloaded / u.total) * 100);
      v.status_color = getStatusColor(u.status);
    }
    if (u.status === 'Completed' || u.status === '已完成') completedCount++;
    if (u.status === 'Completed' || u.status === 'Failed') {
      isDownloading = videos.some(x => x.status === 'Downloading' || x.status === 'Resolving');
      if (!isDownloading) statusText = '下载完成 (' + doneCount + '/' + totalCount + ')';
    }
  }

  async function copyCommand() {
    isParsing = true; errorBox = ''; parseProgress = 20;
    try { 
      await invoke('copy_get_lesson_record_info'); 
      parseProgress = 100;
      statusText = '筛选文本已成功复制到剪贴板';
    }
    catch (e) { 
      errorBox = '复制失败，请手动复制命令'; 
      statusText = '复制失败';
    }
    finally { isParsing = false; parseProgress = 0; }
  }

  async function parseFromClipboard() {
    isParsing = true; errorBox = ''; parseProgress = 20;
    try { 
      const clip = await invoke<string>('read_clipboard'); 
      parseProgress = 60; 
      await doParse(clip); 
    }
    catch (e) { 
      errorBox = '读取剪贴板失败：' + String(e); 
      statusText = '解析失败'; 
    }
    finally { isParsing = false; parseProgress = 0; }
  }

  async function doParse(content: string) {
    parseProgress = 70;
    const parsed = await invoke<VideoItem[]>('parse_videos', { content, maxConcurrent });
    videos = parsed.map((v, i) => ({ ...v, display_index: v.display_index || String(i + 1) }));
    parseProgress = 100;
    if (parsed.length === 0) { errorBox = '未在内容中找到视频链接'; statusText = '未找到视频'; }
    else { statusText = '已解析 ' + parsed.length + ' 个视频'; errorBox = ''; }
  }

  async function startDownloads() {
    if (videos.length === 0) { errorBox = '没有可下载的视频'; return; }
    isDownloading = true; errorBox = ''; statusText = '下载中...';
    try { await invoke('start_downloads', { videos, dir: downloadDir, maxConcurrent }); }
    catch (e) { errorBox = '下载错误: ' + String(e); statusText = '下载失败'; }
  }

  async function cancelAll() { try { await invoke('cancel_all_downloads'); statusText = '已请求取消'; } catch (e) { console.error(e); } }
  async function cancelOne(id: number) { try { await invoke('cancel_download', { videoId: id }); } catch (e) { console.error(e); } }
  async function pauseOne(id: number) { try { await invoke('pause_download', { videoId: id }); } catch (e) { console.error(e); } }
  async function resumeOne(id: number) { try { await invoke('resume_download', { videoId: id }); } catch (e) { console.error(e); } }
  async function selectDirectory() { try { downloadDir = await invoke<string>('select_directory'); void persistSettings(); } catch (e) { console.error(e); } }
  async function openFolder() { try { await invoke('open_folder', { path: downloadDir }); } catch (e) { console.error(e); } }
  async function copyUrl(url: string) { try { await invoke('write_clipboard', { text: url }); statusText = '已复制链接'; } catch (e) { console.error(e); } }

  function removeVideo(id: number) { videos = videos.filter(v => v.id !== id); }
  function clearCompleted() { videos = videos.filter(v => !isDone(v.status)); }
  function selectAll() { videos.forEach(v => (v.selected = true)); }
  function clearSelection() { videos.forEach(v => (v.selected = false)); }

  function handleRowClick(video: VideoItem, index: number, e: MouseEvent) {
    if (e.ctrlKey || e.metaKey) { video.selected = !video.selected; }
    else if (e.shiftKey && lastSelectedIndex >= 0) {
      const [s, en] = lastSelectedIndex < index ? [lastSelectedIndex, index] : [index, lastSelectedIndex];
      for (let i = s; i <= en; i++) videos[i].selected = true;
    } else {
      videos.forEach(v => (v.selected = false));
      video.selected = true;
    }
    lastSelectedIndex = index;
  }

  function startResize(col: keyof ColumnWidths, e: MouseEvent) {
    e.preventDefault(); e.stopPropagation();
    draggingCol = col as string;
    dragStartX = e.clientX;
    dragStartW = widths[col];
    const onMove = (ev: MouseEvent) => {
      if (!draggingCol) return;
      const delta = ev.clientX - dragStartX;
      widths = { ...widths, [draggingCol]: Math.max(40, Math.round(dragStartW + delta)) };
    };
    const onUp = () => {
      draggingCol = null;
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
      void persistSettings();
    };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }

  function onContextMenu(e: MouseEvent, video: VideoItem | null) { e.preventDefault(); contextMenu = { x: e.clientX, y: e.clientY, video }; }
  function closeContextMenu() { contextMenu = null; }

  function formatSize(bytes: number): string {
    if (!bytes) return '0 B';
    if (bytes >= 1 << 30) return (bytes / (1 << 30)).toFixed(2) + ' GB';
    if (bytes >= 1 << 20) return (bytes / (1 << 20)).toFixed(2) + ' MB';
    if (bytes >= 1 << 10) return (bytes / (1 << 10)).toFixed(2) + ' KB';
    return bytes + ' B';
  }
  function formatSpeed(bps: number): string {
    if (!bps) return '';
    if (bps >= 1 << 20) return (bps / (1 << 20)).toFixed(2) + ' MB/s';
    if (bps >= 1 << 10) return (bps / (1 << 10)).toFixed(2) + ' KB/s';
    return bps.toFixed(2) + ' B/s';
  }
  function formatTime(seconds: number): string {
    if (!seconds) return '--:--:--';
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = Math.floor(seconds % 60);
    return [h, m, s].map(n => String(n).padStart(2, '0')).join(':');
  }
  async function initializeTool() {
    try {
      await invoke('initialize_tool');
      statusText = '初始化完成';
      appendLogSafe('解析工具初始化成功');
    } catch (e) {
      appendLogSafe(`初始化失败: ${String(e)}`);
    }
  }
  function appendLogSafe(text: string) {
    logs = [...logs, text];
  }
</script>
<!-- Context Menu -->
{#if contextMenu}
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<div class="ctx-backdrop" role="presentation" onclick={closeContextMenu} onkeydown={(e) => { if (e.key === 'Escape') closeContextMenu(); }}></div>
<div class="context-menu" style="top:{contextMenu.y}px;left:{contextMenu.x}px">
  {#if contextMenu.video}
    <button class="ctx-item" role="menuitem" onclick={() => { pauseOne(contextMenu?.video?.id ?? 0); closeContextMenu(); }}>⏸ 暂停</button>
    <button class="ctx-item" role="menuitem" onclick={() => { resumeOne(contextMenu?.video?.id ?? 0); closeContextMenu(); }}>▶ 恢复</button>
    <button class="ctx-item" role="menuitem" onclick={() => { copyUrl(contextMenu?.video?.url ?? ""); closeContextMenu(); }}>🔗 复制链接</button>
    <div class="ctx-sep"></div>
    <button class="ctx-item danger" role="menuitem" onclick={() => { removeVideo(contextMenu?.video?.id ?? 0); closeContextMenu(); }}>🗑 删除</button>
  {:else}
    <button class="ctx-item" role="menuitem" onclick={() => { selectAll(); closeContextMenu(); }}>全选</button>
    <button class="ctx-item" role="menuitem" onclick={() => { clearSelection(); closeContextMenu(); }}>取消选中</button>
    <div class="ctx-sep"></div>
    <button class="ctx-item" role="menuitem" onclick={() => { clearCompleted(); closeContextMenu(); }}>清除已完成</button>
  {/if}
</div>
{/if}

<div class="app">
  <header class="header">
    <div class="header-title">
      <span class="title-text">Classin视频下载工具</span>
    </div>
  </header>

  <div class="main-content">
    <!-- Left Panel: Buttons + Settings -->
    <aside class="left-panel">
      <div class="panel-section">
        <h3 class="panel-title">操作步骤</h3>
        <div class="step-buttons">
          <button class="step-btn primary-btn" class:active={isParsing} onclick={copyCommand} disabled={isParsing}>1. 复制筛选关键词至剪贴板</button>
          <button class="step-btn teal-btn" class:active={isParsing} onclick={parseFromClipboard} disabled={isParsing}>2. 解析剪贴板中的请求头</button>
          <button class="step-btn success-btn" class:active={isDownloading} onclick={startDownloads} disabled={videos.length === 0 || isDownloading}>3. 下载全部视频</button>
          <button class="step-btn purple-btn" onclick={openFolder}>4. 打开下载目录</button>
        </div>
      </div>

      <div class="panel-section settings-section">
        <h3 class="panel-title">下载设置</h3>
        <div class="slider-group">
          <label for="max-concurrent">最大同时下载视频数量</label>
          <input id="max-concurrent" type="range" bind:value={maxConcurrent} min={1} max={16} step={1} oninput={persistSettings} />
          <span class="slider-value">当前值: {maxConcurrent}</span>
        </div>
        <div class="slider-group">
          <label for="max-threads">最大下载可用线程数量</label>
          <input id="max-threads" type="range" bind:value={maxThreads} min={1} max={256} step={1} oninput={persistSettings} />
          <span class="slider-value">当前值: {maxThreads}</span>
        </div>

        <button class="step-btn orange-btn" onclick={initializeTool}>初始化解析工具</button>

        <div class="download-path-section">
          <label for="download-dir-display">下载目录：</label>
          <div class="path-control">
            <span id="download-dir-display" class="download-path" title={downloadDir}>{downloadDir || '下载目录'}</span>
            <button class="btn-change" onclick={selectDirectory}>更改</button>
          </div>
        </div>
      </div>

      {#if errorBox}
        <div class="error-box">
          <span class="error-icon">⚠️</span>
          <span>{errorBox}</span>
        </div>
      {/if}
    </aside>

    <!-- Right Panel: Table + Logs -->
    <main class="right-panel">
      <div class="table-section">
        <div class="table-header">
          <div class="table-info">
            <span>已选择 <b>{selectedCount}</b> / {totalCount} 个</span>
            <span class="success">✅ 成功 {doneCount}</span>
            <span class="fail">❌ 失败 {failedCount}</span>
          </div>
          <div class="table-actions">
            <button class="small" onclick={selectAll}>全选</button>
            <button class="small" onclick={clearSelection}>取消选择</button>
            <button class="small" onclick={clearCompleted}>清除已完成</button>
          </div>
        </div>
        <div class="table-container" role="region" aria-label="视频列表" oncontextmenu={(e) => onContextMenu(e, null)}>
          {#if videos.length === 0}
            <div class="empty-table">
              <p>暂无视频数据，请先执行步骤 1-2 解析视频链接</p>
            </div>
          {:else}
            <table class="video-table">
              <thead>
                <tr>
                  <th style="width:{widths.idx}px"><div class="col-header">序号</div><!-- svelte-ignore a11y_no_noninteractive_element_interactions --><div class="col-resize" role="separator" aria-orientation="vertical" onmousedown={(e) => startResize('idx', e)}></div></th>
                  <th style="width:{widths.name}px"><div class="col-header">课程名称</div><!-- svelte-ignore a11y_no_noninteractive_element_interactions --><div class="col-resize" role="separator" aria-orientation="vertical" onmousedown={(e) => startResize('name', e)}></div></th>
                  <th style="width:{widths.size}px"><div class="col-header">文件大小</div><!-- svelte-ignore a11y_no_noninteractive_element_interactions --><div class="col-resize" role="separator" aria-orientation="vertical" onmousedown={(e) => startResize('size', e)}></div></th>
                  <th style="width:{widths.progress}px"><div class="col-header">进度</div><!-- svelte-ignore a11y_no_noninteractive_element_interactions --><div class="col-resize" role="separator" aria-orientation="vertical" onmousedown={(e) => startResize('progress', e)}></div></th>
                  <th style="width:{widths.status}px"><div class="col-header">状态</div><!-- svelte-ignore a11y_no_noninteractive_element_interactions --><div class="col-resize" role="separator" aria-orientation="vertical" onmousedown={(e) => startResize('status', e)}></div></th>
                  <th style="width:{widths.action}px"><div class="col-header">操作</div><!-- svelte-ignore a11y_no_noninteractive_element_interactions --><div class="col-resize" role="separator" aria-orientation="vertical" onmousedown={(e) => startResize('action', e)}></div></th>
                </tr>
              </thead>
              <tbody>
                {#each sortedVideos as video, i (video.id)}
                  <tr class:selected={video.selected ?? false} oncontextmenu={(e) => { e.stopPropagation(); onContextMenu(e, video); }} onclick={(e) => handleRowClick(video, i, e)}>
                    <td style="width:{widths.idx}px"><div class="vid-num">{video.display_index}</div></td>
                    <td style="width:{widths.name}px" title={video.url}><div class="vid-name">{video.name}</div></td>
                    <td style="width:{widths.size}px">{formatSize(video.file_size)}</td>
                    <td style="width:{widths.progress}px">
                      <div class="progress-wrap">
                        <progress value={video.progress} max="100"></progress>
                        <span class="pct">{video.progress.toFixed(1)}%</span>
                      </div>
                    </td>
                    <td style="width:{widths.status}px"><span class="badge" style="background:{getStatusColor(video.status)}">{video.status}</span></td>
                    <td style="width:{widths.action}px">
                      <div class="actions">
                        <button class="btn-sm btn-copy" onclick={() => copyUrl(video.url)}>复制URL</button>
                        <button class="btn-sm btn-resume" onclick={() => resumeOne(video.id)} title="下载" disabled={isDone(video.status)}>下载</button>
                        <button class="btn-sm btn-cancel" onclick={() => removeVideo(video.id)} title="删除">删除</button>
                      </div>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {/if}
        </div>
      </div>

      <div class="logs-section">
        <div class="logs-header">
          <h3>日志与输出</h3>
          <div class="tab-controls">
            <button class="tab-btn active">解析日志</button>
          </div>
        </div>
        <div class="log-content">{#each logs as log (log)}<div class="log-line">{log}</div>{/each}</div>
      </div>
    </main>
  </div>

  <div class="status-footer">
    <div class="footer-left">
      <span>解析状态：</span>
      <div class="footer-progress"><div class="footer-bar" style="width:{parseProgress}%"></div></div>
      <span>{statusText}</span>
    </div>
    <div class="footer-right">
      <span>总速度：</span>
      <span class="speed-value">{formatSpeed(overallSpeed)}</span>
      <span>已完成：</span>
      <span class="completed-count">{completedCount}/{totalCount}</span>
    </div>
  </div>
</div>
<style>
/* ── WPF Exact Theme Colors ── */
:root {
  /* Backgrounds */
  --bg-window: #F0F2F5;
  --bg-panel: #FFFFFF;
  --bg-log: #F8F9FA;
  --bg-hover: #F5F5F5;

  /* Borders */
  --border-main: #D9D9D9;
  --border-light: #E0E0E0;
  --border-dark: #BDBDBD;

  /* Text Colors */
  --text-primary: #333333;
  --text-secondary: #616161;
  --text-muted: #9E9E9E;

  /* Vibrant Colors (from WPF) */
  --color-blue: #2196F3;
  --color-green: #4CAF50;
  --color-red: #F44336;
  --color-purple: #9C27B0;
  --color-teal: #00BCD4;
  --color-orange: #FF9800;
  --color-accent: #4A90E2;

  /* Status Colors */
  --status-done: #4CAF50;
  --status-fail: #F44336;
  --status-cancel: #9E9E9E;
  --status-pause: #FF9800;
  --status-partial: #00BCD4;
  --status-downloading: #2196F3;
  --status-resolving: #9C27B0;
  --status-waiting: #FF9800;

  /* Shadows & Radius */
  --shadow-sm: 0 1px 3px rgba(0,0,0,0.1);
  --shadow-md: 0 2px 6px rgba(0,0,0,0.12);
  --radius-lg: 8px;
  --radius-md: 6px;
  --radius-sm: 4px;

  /* Typography */
  --font-family: "Segoe UI", "Microsoft YaHei", "PingFang SC", sans-serif;
  --font-size-xl: 19px;
  --font-size-lg: 16px;
  --font-size-md: 13px;
  --font-size-sm: 12px;
  --font-size-xs: 11px;
}


/* Reset */
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

/* Main App Container - Matches WPF Window (1280x720, bg #F0F2F5) */
.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--bg-window);
  color: var(--text-primary);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
  overflow: hidden;
}

/* Header - White panel */
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 20px;
  background: var(--bg-panel);
  border-bottom: 1px solid var(--border-main);
  flex-shrink: 0;
}

.header-title { display: flex; align-items: center; }
.title-text { font-size: var(--font-size-xl); font-weight: 600; color: var(--color-accent); }

/* Main content: 2-column layout (Panel 1* / Splitter 5px / Result 3*) */
.main-content { display: flex; flex: 1; overflow: hidden; }

/* Left Panel - WPF PanelColumn */
.left-panel {
  width: 280px;
  min-width: 240px;
  max-width: 320px;
  background: var(--bg-panel);
  border-right: 1px solid var(--border-main);
  display: flex;
  flex-direction: column;
  padding: 10px;
  gap: 10px;
  overflow-y: auto;
  flex-shrink: 0;
}

/* White card panels with rounded corners (WPF CornerRadius=8) */
.panel-section {
  background: var(--bg-panel);
  border: 1px solid var(--border-main);
  border-radius: var(--radius-lg);
  padding: 10px;
}

.panel-title {
  margin: 0 0 10px 5px;
  font-size: var(--font-size-lg);
  font-weight: bold;
  color: var(--text-primary);
}

.settings-section { display: flex; flex-direction: column; gap: 10px; }

/* Step buttons - WPF: 32px min-height, radius 6, white text, medium weight */
.step-buttons { display: flex; flex-direction: column; gap: 6px; }
.step-btn {
  width: 100%;
  text-align: center;
  padding: 8px 12px;
  min-height: 32px;
  border: none;
  border-radius: var(--radius-md);
  color: white;
  font-family: inherit;
  font-size: var(--font-size-md);
  font-weight: 500;
  cursor: pointer;
  transition: background-color 0.15s ease;
}
.step-btn:disabled { opacity: 0.6; cursor: not-allowed; }

/* Vibrant color variants (WPF SolidColorBrush) with darkened hover/pressed */
.primary-btn { background: var(--color-blue); }
.primary-btn:hover:not(:disabled) { background: #1976D2; }
.primary-btn:active:not(:disabled) { background: #1565C0; }
.teal-btn { background: var(--color-teal); }
.teal-btn:hover:not(:disabled) { background: #00ACC1; }
.teal-btn:active:not(:disabled) { background: #0097A7; }
.success-btn { background: var(--color-green); }
.success-btn:hover:not(:disabled) { background: #43A047; }
.success-btn:active:not(:disabled) { background: #388E3C; }
.purple-btn { background: var(--color-purple); }
.purple-btn:hover:not(:disabled) { background: #8E24AA; }
.purple-btn:active:not(:disabled) { background: #7B1FA2; }
.orange-btn { background: var(--color-orange); }
.orange-btn:hover:not(:disabled) { background: #FB8C00; }
.orange-btn:active:not(:disabled) { background: #F57C00; }

/* Sliders - WPF Slider styling */
.slider-group { display: flex; flex-direction: column; gap: 4px; }
.slider-group label { font-size: var(--font-size-md); color: var(--text-primary); }
.slider-group input[type="range"] {
  width: 100%;
  height: 4px;
  -webkit-appearance: none;
  appearance: none;
  background: var(--border-main);
  border-radius: 2px;
  outline: none;
  margin: 5px 0;
}
.slider-group input[type="range"]::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 14px;
  height: 14px;
  background: var(--color-blue);
  border-radius: 50%;
  cursor: pointer;
}
.slider-value { font-size: var(--font-size-md); color: var(--text-primary); }

/* Download path control */
.download-path-section { display: flex; flex-direction: column; gap: 4px; }
.download-path-section label { font-size: var(--font-size-md); color: var(--text-primary); }
.path-control { display: flex; align-items: center; gap: 6px; }
.download-path {
  flex: 1;
  font-size: var(--font-size-sm);
  color: var(--text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Change button - WPF CompactSmallButtonStyle (min 50px, padding 4,1) */
.btn-change {
  padding: 4px 10px;
  min-width: 50px;
  min-height: 26px;
  font-size: var(--font-size-sm);
  font-family: inherit;
  background: transparent;
  border: 1px solid var(--border-main);
  border-radius: var(--radius-sm);
  color: var(--color-accent);
  cursor: pointer;
  white-space: nowrap;
}
.btn-change:hover { border-color: var(--color-accent); background: var(--bg-hover); }

/* Error box */
.error-box {
  background: rgba(244,67,54,0.1);
  border: 1px solid var(--color-red);
  color: var(--color-red);
  border-radius: var(--radius-sm);
  padding: 8px 12px;
  font-size: var(--font-size-sm);
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}
.error-icon { font-size: 14px; }

/* Right Panel - WPF ResultColumn (3*) */
.right-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  gap: 10px;
}

/* Table Section */
.table-section {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  margin: 10px;
  background: var(--bg-panel);
  border-radius: var(--radius-lg);
  border: 1px solid var(--border-main);
  overflow: hidden;
}

.table-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  background: var(--bg-panel);
  border-bottom: 1px solid var(--border-main);
  flex-shrink: 0;
}

.table-info {
  display: flex;
  gap: 12px;
  font-size: var(--font-size-sm);
  align-items: center;
}
.table-info .success { color: var(--status-done); font-weight: 600; }
.table-info .fail { color: var(--status-fail); font-weight: 600; }

.table-actions {
  display: flex;
  gap: 4px;
}

/* Table */
.table-container {
  flex: 1;
  overflow: auto;
}

.video-table {
  width: 100%;
  border-collapse: collapse;
  table-layout: fixed;
}

.video-table th, .video-table td {
  padding: 8px 5px;
  border-bottom: 1px solid var(--border-light);
  vertical-align: middle;
  text-align: center;
}

.video-table thead th {
  background: var(--bg-panel);
  position: sticky;
  top: 0;
  z-index: 2;
  font-weight: bold;
  color: var(--text-primary);
  font-size: var(--font-size-sm);
  padding: 8px 5px;
}

.col-header {
  display: inline-block;
  font-weight: bold;
  cursor: pointer;
}
.col-header:hover { color: var(--color-accent); }
.col-resize {
  position: absolute;
  right: 0;
  top: 0;
  bottom: 0;
  width: 5px;
  cursor: col-resize;
  background: transparent;
}
.col-resize:hover { background: var(--border-main); }

.video-table tr:hover { background: var(--bg-hover); }
.video-table tr.selected { background: rgba(33,150,243,0.1); }

.vid-num {
  font-weight: 600;
  color: var(--text-muted);
  font-size: var(--font-size-xs);
  text-align: center;
  min-width: 60px;
}
.vid-name {
  font-size: var(--font-size-sm);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: left;
}
/* Progress bar - WPF style (height 12px) */
.progress-wrap { display: flex; align-items: center; gap: 4px; }
progress {
  flex: 1;
  height: 12px;
  border: none;
  background: var(--border-main);
  border-radius: 6px;
  overflow: hidden;
}
progress::-webkit-progress-bar, progress::-moz-progress-bar {
  background: var(--border-main);
  border-radius: 6px;
}
progress::-webkit-progress-value, progress::-moz-progress-bar {
  background: linear-gradient(90deg, var(--color-blue), var(--color-green));
  height: 12px;
  border-radius: 6px;
  transition: width 0.3s;
}
.pct {
  font-size: var(--font-size-xs);
  color: var(--text-muted);
  min-width: 35px;
  text-align: right;
}

/* Status badge */
.badge {
  display: inline-block;
  padding: 2px 6px;
  border-radius: 3px;
  color: white;
  font-size: var(--font-size-xs);
  font-weight: 500;
  text-align: center;
}

/* Action buttons - match WPF gray button style */
.actions { display: flex; gap: 3px; }
.btn-sm {
  padding: 2px 6px;
  font-size: var(--font-size-xs);
  border-radius: var(--radius-sm);
  border: 1px solid var(--border-main);
  background: var(--bg-panel);
  cursor: pointer;
  min-height: 20px;
  transition: background 0.1s ease;
  font-family: inherit;
}
.btn-sm:hover { background: var(--bg-hover); }
.btn-copy { color: var(--color-accent); border-color: var(--color-accent); }
.btn-resume { color: var(--color-green); border-color: var(--color-green); }
.btn-cancel { color: var(--color-red); border-color: var(--color-red); }

/* Empty table state */
.empty-table {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 200px;
  color: var(--text-muted);
  font-size: var(--font-size-md);
  padding: 40px;
  text-align: center;
}
.empty-table p {
  margin: 0;
}

.logs-section {
  background: var(--bg-panel);
  border: 1px solid var(--border-main);
  border-radius: var(--radius-lg);
  padding: 8px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  height: 180px;
  margin: 0 10px 10px 10px;
}
.logs-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px;
  margin: 0;
}
.logs-header h3 {
  margin: 0;
  font-size: var(--font-size-md);
  font-weight: bold;
  color: var(--text-primary);
}
.tab-controls {
  display: flex;
  gap: 4px;
}
.tab-btn {
  padding: 4px 12px;
  font-size: var(--font-size-sm);
  background: transparent;
  border: 1px solid transparent;
  border-radius: var(--radius-sm);
  cursor: pointer;
  color: var(--text-secondary);
}
.tab-btn.active {
  background: var(--color-blue);
  color: white;
  border-color: var(--color-blue);
}
.log-content {
  flex: 1;
  overflow-y: auto;
  font-family: Consolas, "Courier New", monospace;
  font-size: var(--font-size-sm);
  padding: 8px;
  background: var(--bg-log);
  color: var(--text-primary);
}
.log-line {
  padding: 2px 8px;
  border-bottom: 1px solid var(--border-light);
  white-space: pre-wrap;
  word-break: break-all;
}

/* Status Footer - WPF StatusBar (30px height) */
.status-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  background: var(--bg-panel);
  border-top: 1px solid var(--border-main);
  font-size: var(--font-size-sm);
  color: var(--text-primary);
  flex-shrink: 0;
  height: 30px;
}
.footer-left, .footer-right {
  display: flex;
  align-items: center;
  gap: 8px;
}
.footer-progress {
  width: 120px;
  height: 12px;
  background: var(--border-main);
  border-radius: 6px;
  overflow: hidden;
}
.footer-bar {
  height: 100%;
  background: var(--color-blue);
  transition: width 0.3s;
}
.speed-value { font-weight: 600; color: var(--color-accent); }
.completed-count { font-weight: 600; color: var(--text-secondary); }

/* Context Menu - WPF-style popup */
.ctx-backdrop {
  position: fixed;
  inset: 0;
  z-index: 100;
}
.context-menu {
  position: fixed;
  z-index: 101;
  background: var(--bg-panel);
  border: 1px solid var(--border-main);
  box-shadow: var(--shadow-md);
  border-radius: var(--radius-sm);
  min-width: 160px;
  padding: 4px 0;
  animation: fadeIn 0.12s ease;
}
@keyframes fadeIn {
  from { opacity: 0; transform: translateY(-4px); }
  to { opacity: 1; transform: none; }
}
.ctx-item {
  display: block;
  width: 100%;
  text-align: left;
  padding: 6px 16px;
  cursor: pointer;
  font-size: var(--font-size-sm);
  color: var(--text-primary);
  background: transparent;
  border: none;
  border-radius: 0;
}
.ctx-item:hover { background: var(--bg-hover); }
.ctx-item.danger { color: var(--color-red); }
.ctx-sep { height: 1px; background: var(--border-main); margin: 4px 0; }

/* Scrollbar styling */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}
::-webkit-scrollbar-track {
  background: var(--bg-window);
}
::-webkit-scrollbar-thumb {
  background: var(--border-dark);
  border-radius: 4px;
}
::-webkit-scrollbar-thumb:hover {
  background: var(--border-main);
}
</style>
