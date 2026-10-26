<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';

  interface VideoItem {
    id: number; display_index: string; name: string; url: string;
    file_size: number; downloaded_bytes: number; status: string;
    progress: number; current_speed: number; remaining_time_secs: number;
    phase: string; error_message: string; selected?: boolean;
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

  const DEFAULT_WIDTHS: ColumnWidths = { idx: 70, name: 280, size: 110, progress: 240, status: 160, action: 130 };

  let videos = $state<VideoItem[]>([]);
  let statusText = $state('就绪');
  let downloadDir = $state('');
  let maxConcurrent = $state(5);
  let isParsing = $state(false);
  let isDownloading = $state(false);
  let errorBox = $state('');
  let logs = $state<string[]>([]);
  let inputText = $state('');
  let theme = $state<'light' | 'dark'>('light');
  let widths = $state<ColumnWidths>({ ...DEFAULT_WIDTHS });
  let sortCol = $state('');
  let sortAsc = $state(true);
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

  const sortedVideos = $derived.by(() => {
    if (!sortCol) return videos;
    const col = sortCol;
    const dir = sortAsc ? 1 : -1;
    return [...videos].sort((a, b) => {
      let av: number | string = 0;
      let bv: number | string = 0;
      if (col === 'idx') { av = a.display_index; bv = b.display_index; }
      else if (col === 'name') { av = a.name.toLowerCase(); bv = b.name.toLowerCase(); }
      else if (col === 'size') { av = a.file_size; bv = b.file_size; }
      else if (col === 'progress') { av = a.progress; bv = b.progress; }
      else if (col === 'status') { av = a.status; bv = b.status; }
      if (av < bv) return -1 * dir;
      if (av > bv) return 1 * dir;
      return 0;
    });
  });

  const totalCount = $derived(videos.length);
  const doneCount = $derived(videos.filter(v => isDone(v.status)).length);
  const failedCount = $derived(videos.filter(v => v.status.toLowerCase().includes('fail')).length);
  const activeCount = $derived(videos.filter(v => v.status.toLowerCase().includes('down')).length);
  const selectedCount = $derived(videos.filter(v => v.selected).length);

  function isDone(s: string) {
    const l = s.toLowerCase();
    return l.includes('done') || l.includes('complete') || l.includes('已完成');
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
      theme = s.theme === 'dark' ? 'dark' : 'light';
      if (s.column_widths && (s.column_widths.idx || 0) > 0) {
        widths = { ...DEFAULT_WIDTHS, ...s.column_widths } as ColumnWidths;
      }
      if (s.sort && s.sort.column) { sortCol = s.sort.column; sortAsc = s.sort.ascending; }
      if (s.max_concurrent > 0) maxConcurrent = s.max_concurrent;
      if (s.download_dir) downloadDir = s.download_dir;
    } catch (e) { console.error(e); }
    applyTheme();
  }

  async function persistSettings() {
    const payload: AppSettings = {
      theme,
      column_widths: widths,
      sort: { column: sortCol, ascending: sortAsc },
      max_concurrent: Number(maxConcurrent),
      download_dir: downloadDir,
    };
    try { await invoke('save_settings', { newSettings: payload }); }
    catch (e) { console.error(e); }
  }

  function applyTheme() { document.documentElement.setAttribute('data-theme', theme); }

  function toggleTheme() {
    theme = theme === 'light' ? 'dark' : 'light';
    applyTheme();
    void persistSettings();
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
    }
    if (u.status === 'Completed' || u.status === '已完成') completedCount++;
    if (u.status === 'Completed' || u.status === 'Failed') {
      isDownloading = videos.some(x => x.status === 'Downloading' || x.status === 'Resolving');
      if (!isDownloading) statusText = '下载完成 (' + doneCount + '/' + totalCount + ')';
    }
  }

  async function parseFromClipboard() {
    isParsing = true; errorBox = ''; parseProgress = 20;
    try { const clip = await invoke<string>('read_clipboard'); parseProgress = 60; await doParse(clip); }
    catch (e) { errorBox = '读取剪贴板失败: ' + String(e); statusText = '解析失败'; }
    finally { isParsing = false; parseProgress = 0; }
  }

  async function parseFromInput() {
    if (!inputText.trim()) return;
    isParsing = true; errorBox = ''; parseProgress = 30;
    try { await doParse(inputText); }
    catch (e) { errorBox = '解析失败: ' + String(e); statusText = '解析失败'; }
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
  async function copyCommand() { try { await invoke('copy_get_lesson_record_info'); statusText = '已复制命令'; } catch (e) { errorBox = '复制失败: ' + String(e); } }
  async function copyUrl(url: string) { try { await invoke('write_clipboard', { text: url }); statusText = '已复制链接'; } catch (e) { console.error(e); } }
  async function toggleClipboardWatch() {
    clipboardWatch = !clipboardWatch;
    try { if (clipboardWatch) await invoke('start_clipboard_watch'); else await invoke('stop_clipboard_watch'); } catch (e) { console.error(e); }
  }

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

  function sortBy(col: string) { if (sortCol === col) sortAsc = !sortAsc; else { sortCol = col; sortAsc = true; } void persistSettings(); }

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

  function getStatusColor(status: string): string {
    const s = status.toLowerCase();
    if (s.includes('done') || s.includes('complete') || s.includes('已完成')) return '#4CAF50';
    if (s.includes('fail') || s.includes('error') || s.includes('失败')) return '#F44336';
    if (s.includes('cancel') || s.includes('取消')) return '#9E9E9E';
    if (s.includes('pause') || s.includes('暂停')) return '#FF9800';
    if (s.includes('partial') || s.includes('部分')) return '#00BCD4';
    if (s.includes('down') || s.includes('下载')) return '#2196F3';
    if (s.includes('resolv') || s.includes('解析')) return '#9C27B0';
    return '#607D8B';
  }

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
    <div class="header-left">
      <h1>ClassIn 视频下载器 v2.0</h1>
      <span class="version">Rust + Tauri</span>
    </div>
    <div class="header-right">
      <button class="icon-btn" onclick={toggleTheme} title="切换主题">{theme === 'light' ? '🌙' : '☀️'}</button>
    </div>
  </header>

  <div class="global-status" role="region" aria-label="状态栏">
    <div class="global-status-left">
      <span class="status-label">状态：</span>
      <span class="status-value">{statusText}</span>
      {#if isParsing}<span class="spinner"></span>{/if}
      {#if parseProgress > 0}
        <div class="mini-progress" role="progressbar" aria-valuenow={parseProgress} aria-valuemin="0" aria-valuemax="100"><div class="mini-bar" style="width:{parseProgress}%"></div></div>
      {/if}
    </div>
    <div class="global-status-right">
      <span>速率：{formatSpeed(overallSpeed)}</span>
      <span>完成：{completedCount}</span>
      <span>总计：{totalCount}</span>
    </div>
  </div>

  <div class="toolbar">
    <button class="primary" onclick={parseFromClipboard} disabled={isParsing}>📋 解析剪贴板</button>
    <button class="secondary" onclick={() => inputText = ''}>清空输入</button>
    <button class="primary" onclick={startDownloads} disabled={isDownloading || videos.length === 0}>⬇ 开始下载</button>
    <button class="danger" onclick={cancelAll} disabled={!isDownloading}>⏹ 取消全部</button>
    <button class="secondary" onclick={copyCommand}>复制命令</button>
    <label class="toggle-label"><input type="checkbox" checked={clipboardWatch} onchange={toggleClipboardWatch} />剪贴板监听</label>
  </div>

  <div class="input-area">
    <textarea bind:value={inputText} placeholder="粘贴 ClassIn 视频链接或 JSON 数据..."></textarea>
    <button onclick={parseFromInput} disabled={isParsing || !inputText.trim()}>📄 解析输入</button>
  </div>

  <div class="settings">
    <label><span class="setting-label">最大并发：</span><input type="number" bind:value={maxConcurrent} min="1" max="50" step="1" /></label>
    <label><span class="setting-label">下载目录：</span><span class="path">{downloadDir || '未设置'}</span><button class="small" onclick={selectDirectory}>选择</button><button class="small" onclick={openFolder}>打开目录</button></label>
  </div>

  {#if errorBox}<div class="error">{errorBox}</div>{/if}

  {#if videos.length > 0}
  <div class="table-wrap">
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
      <table class="video-table">
        <thead>
          <tr>
            <th style="width:{widths.idx}px"><button class="col-header" type="button" onclick={() => sortBy('idx')} onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); sortBy('idx'); } }}>#</button><!-- svelte-ignore a11y_no_noninteractive_element_interactions --><div class="col-resize" role="separator" aria-orientation="vertical" onmousedown={(e) => startResize('idx', e)}></div></th>
            <th style="width:{widths.name}px"><button class="col-header" type="button" onclick={() => sortBy('name')} onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); sortBy('name'); } }}>名称 {sortCol === 'name' ? (sortAsc ? '↑' : '↓') : ''}</button><!-- svelte-ignore a11y_no_noninteractive_element_interactions --><div class="col-resize" role="separator" aria-orientation="vertical" onmousedown={(e) => startResize('name', e)}></div></th>
            <th style="width:{widths.size}px"><button class="col-header" type="button" onclick={() => sortBy('size')} onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); sortBy('size'); } }}>大小 {sortCol === 'size' ? (sortAsc ? '↑' : '↓') : ''}</button><!-- svelte-ignore a11y_no_noninteractive_element_interactions --><div class="col-resize" role="separator" aria-orientation="vertical" onmousedown={(e) => startResize('size', e)}></div></th>
            <th style="width:{widths.progress}px">进度</th>
            <th style="width:{widths.status}px"><button class="col-header" type="button" onclick={() => sortBy('status')} onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); sortBy('status'); } }}>状态</button><!-- svelte-ignore a11y_no_noninteractive_element_interactions --><div class="col-resize" role="separator" aria-orientation="vertical" onmousedown={(e) => startResize('status', e)}></div></th>
            <th style="width:{widths.action}px">操作</th>
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
                  {#if video.downloaded_bytes > 0}<br/><small>{formatSize(video.downloaded_bytes)} / {formatSize(video.file_size)}</small>{/if}
                </div>
              </td>
              <td style="width:{widths.status}px"><span class="badge" style="background:{getStatusColor(video.status)}">{video.status}</span>{#if video.current_speed > 0}<br/><small>{formatSpeed(video.current_speed)}</small>{/if}{#if video.phase}<small>{video.phase}</small>{/if}</td>
              <td style="width:{widths.action}px">
                <div class="actions">
                  {#if video.status === 'Paused' || video.status === '已暂停'}<button class="btn-sm btn-resume" onclick={() => resumeOne(video.id)} title="恢复">▶</button>
                  {:else if video.status.toLowerCase().includes('down') || video.status.toLowerCase().includes('resolv')}<button class="btn-sm btn-pause" onclick={() => pauseOne(video.id)} title="暂停">⏸</button>
                  {/if}
                  <button class="btn-sm btn-cancel" onclick={() => cancelOne(video.id)} title="取消" disabled={isDone(video.status)}>✕</button>
                  <button class="btn-sm btn-copy" onclick={() => copyUrl(video.url)} title="复制链接">🔗</button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
  {/if}

  {#if logs.length > 0}
  <div class="logs">
    <div class="logs-header"><h3>📋 日志</h3><button class="small" onclick={() => logs = []}>清空</button></div>
    <div class="log-content">{#each logs.slice(-80).reverse() as log}<div class="log-line">{log}</div>{/each}</div>
  </div>
  {/if}

  <div class="status-footer">
    <div class="footer-left"><span>解析状态：</span><div class="footer-progress"><div class="footer-bar" style="width:{videos.filter(v=>!isDone(v.status)).length?100:parseProgress||0}%"></div></div></div>
    <div class="footer-right"><span>下载目录：</span><span class="path">{downloadDir}</span><button class="small" onclick={selectDirectory}>更改</button></div>
  </div>
</div>
<style>
/* ── Theme Variables ── */
:root {
  --bg-primary: #ffffff; --bg-secondary: #f8f9fa; --bg-tertiary: #f0f2f5;
  --bg-card: #ffffff; --bg-hover: #f5f5f5; --border: #e0e0e0; --border-hover: #bdbdbd;
  --text-primary: #212121; --text-secondary: #616161; --text-muted: #9e9e9e;
  --accent: #2196F3; --accent-hover: #1976D2;
  --success: #4CAF50; --danger: #F44336; --warning: #FF9800;
  --info: #00BCD4; --purple: #9C27B0; --gray: #607D8B;
  --shadow: 0 1px 4px rgba(0,0,0,0.08); --shadow-lg: 0 4px 16px rgba(0,0,0,0.12);
  --radius: 8px; --radius-sm: 4px;
}
:global([data-theme="dark"]) {
  --bg-primary: #0f172a; --bg-secondary: #1e293b; --bg-tertiary: #334155;
  --bg-card: #1e293b; --bg-hover: #2d3f54; --border: #334155; --border-hover: #475569;
  --text-primary: #f1f5f9; --text-secondary: #94a3b8; --text-muted: #64748b;
  --accent: #3b82f6; --accent-hover: #60a5fa;
  --success: #22c55e; --danger: #ef4444; --warning: #f59e0b;
  --info: #06b6d4; --purple: #a855f7; --gray: #64748b;
  --shadow: 0 1px 4px rgba(0,0,0,0.3); --shadow-lg: 0 4px 16px rgba(0,0,0,0.4);
}
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
.app { display: flex; flex-direction: column; height: 100vh; background: var(--bg-primary); color: var(--text-primary); font-family: "Segoe UI","PingFang SC","Microsoft YaHei",sans-serif; font-size: 13px; overflow: hidden; }

.header { display: flex; align-items: center; justify-content: space-between; padding: 10px 16px; background: var(--bg-secondary); border-bottom: 1px solid var(--border); flex-shrink: 0; }
.header-left h1 { font-size: 16px; font-weight: 600; margin: 0; }
.version { font-size: 11px; color: var(--text-muted); margin-left: 8px; vertical-align: middle; }
.icon-btn { background: none; border: 1px solid var(--border); border-radius: var(--radius-sm); cursor: pointer; padding: 4px 8px; font-size: 14px; transition: all 0.15s; }
.icon-btn:hover { background: var(--bg-tertiary); }

.global-status { display: flex; align-items: center; justify-content: space-between; padding: 6px 16px; background: var(--bg-tertiary); border-bottom: 1px solid var(--border); font-size: 12px; color: var(--text-secondary); flex-shrink: 0; }
.global-status-left, .global-status-right { display: flex; align-items: center; gap: 12px; }
.status-label { font-weight: 600; color: var(--text-primary); }
.status-value { color: var(--accent); font-weight: 500; }
.mini-progress { width: 80px; height: 4px; background: var(--border); border-radius: 2px; overflow: hidden; }
.mini-bar { height: 100%; background: var(--accent); transition: width 0.3s; }
@keyframes spin { to { transform: rotate(360deg); } }
.spinner { display: inline-block; width: 12px; height: 12px; border: 2px solid var(--border); border-top-color: var(--accent); border-radius: 50%; animation: spin 0.8s linear infinite; margin-left: 6px; }

.toolbar { display: flex; align-items: center; gap: 8px; padding: 10px 16px; background: var(--bg-primary); border-bottom: 1px solid var(--border); flex-wrap: wrap; flex-shrink: 0; }
button { padding: 6px 14px; border: 1px solid var(--border); border-radius: var(--radius-sm); cursor: pointer; font-size: 13px; font-family: inherit; transition: all 0.15s; background: var(--bg-secondary); color: var(--text-primary); white-space: nowrap; }
button:hover:not(:disabled) { background: var(--bg-tertiary); border-color: var(--border-hover); }
button:disabled { opacity: 0.5; cursor: not-allowed; }
button.primary { background: var(--accent); color: white; border-color: var(--accent); }
button.primary:hover:not(:disabled) { background: var(--accent-hover); }
button.danger { background: var(--danger); color: white; border-color: var(--danger); }
button.danger:hover:not(:disabled) { background: #d32f2f; }
button.small { padding: 3px 8px; font-size: 11px; }
.toggle-label { display: flex; align-items: center; gap: 4px; font-size: 12px; color: var(--text-secondary); cursor: pointer; }

.input-area { display: flex; gap: 8px; padding: 10px 16px; background: var(--bg-primary); border-bottom: 1px solid var(--border); flex-shrink: 0; }
.input-area textarea { flex: 1; padding: 8px 10px; border: 1px solid var(--border); border-radius: var(--radius-sm); resize: none; font-family: Consolas,monospace; font-size: 12px; background: var(--bg-secondary); color: var(--text-primary); min-height: 60px; outline: none; transition: border-color 0.15s; }
.input-area textarea:focus { border-color: var(--accent); }

.settings { display: flex; align-items: center; gap: 20px; padding: 8px 16px; background: var(--bg-secondary); border-bottom: 1px solid var(--border); flex-shrink: 0; font-size: 12px; }
.setting-label { font-weight: 600; color: var(--text-secondary); }
.settings input[type=number] { width: 60px; padding: 3px 6px; border: 1px solid var(--border); border-radius: var(--radius-sm); background: var(--bg-primary); color: var(--text-primary); }
.path { color: var(--text-muted); max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

.error { margin: 8px 16px; padding: 8px 12px; background: rgba(244,67,54,0.1); border: 1px solid var(--danger); color: var(--danger); border-radius: var(--radius-sm); font-size: 12px; }

.table-wrap { flex: 1; margin: 0 16px; display: flex; flex-direction: column; min-height: 0; overflow: hidden; }
.table-header { display: flex; align-items: center; justify-content: space-between; padding: 6px 8px; background: var(--bg-secondary); border: 1px solid var(--border); border-bottom: none; border-radius: var(--radius) var(--radius) 0 0; }
.table-info, .table-actions { display: flex; gap: 8px; align-items: center; font-size: 12px; }
.table-info .success { color: var(--success); }
.table-info .fail { color: var(--danger); }
.table-container { flex: 1; overflow: auto; border: 1px solid var(--border); border-radius: 0 0 var(--radius) var(--radius); }
.video-table { width: 100%; border-collapse: collapse; table-layout: fixed; }
.video-table th, .video-table td { padding: 6px 8px; text-align: left; border-bottom: 1px solid var(--border); vertical-align: middle; }
.video-table thead { position: sticky; top: 0; z-index: 2; background: var(--bg-tertiary); }
.video-table th { font-weight: 600; color: var(--text-secondary); font-size: 11px; }
.col-header { cursor: pointer; user-select: none; display: flex; align-items: center; gap: 4px; background: none; border: none; padding: 0; font: inherit; color: inherit; }
.col-header:hover { color: var(--accent); }
.col-resize { position: absolute; right: 0; top: 0; bottom: 0; width: 4px; cursor: col-resize; background: transparent; }
.video-table th { position: relative; }
.video-table tr:hover { background: var(--bg-hover); }
.video-table tr.selected { background: rgba(33,150,243,0.1); }
.vid-num { font-weight: 600; color: var(--text-muted); font-size: 11px; text-align: center; }
.vid-name { font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
progress { width: 100%; height: 6px; border: none; background: var(--border); border-radius: 3px; overflow: hidden; }
progress::-webkit-progress-bar { background: var(--border); }
progress::-webkit-progress-value { background: linear-gradient(90deg, var(--accent), var(--success)); transition: width 0.3s; }
progress::-moz-progress-bar { background: linear-gradient(90deg, var(--accent), var(--success)); }
.pct { font-size: 11px; color: var(--text-muted); margin-left: 4px; }
.badge { display: inline-block; padding: 2px 6px; border-radius: 3px; color: white; font-size: 11px; font-weight: 500; }
.actions { display: flex; gap: 3px; }
.btn-sm { padding: 2px 6px; font-size: 11px; border-radius: var(--radius-sm); border: 1px solid var(--border); background: var(--bg-secondary); cursor: pointer; }
.btn-sm:hover { background: var(--bg-tertiary); }
.btn-resume { color: var(--success); border-color: var(--success); }
.btn-pause { color: var(--warning); border-color: var(--warning); }
.btn-cancel { color: var(--danger); border-color: var(--danger); }
.btn-copy { color: var(--accent); border-color: var(--accent); }

.logs { margin: 0 16px 8px; background: #1e1e1e; border-radius: var(--radius-sm); overflow: hidden; flex-shrink: 0; }
.logs-header { display: flex; align-items: center; justify-content: space-between; padding: 8px 12px; background: #2d2d2d; }
.logs-header h3 { margin: 0; color: #e2e2e2; font-size: 13px; }
.logs-header button { background: #3d3d3d; border-color: #555; color: #ccc; font-size: 11px; }
.log-content { max-height: 150px; overflow-y: auto; font-family: Consolas,monospace; font-size: 11px; }
.log-line { padding: 2px 8px; color: #d4d4d4; border-bottom: 1px solid #2d2d2d; white-space: pre-wrap; word-break: break-all; }

.status-footer { display: flex; align-items: center; justify-content: space-between; padding: 4px 16px; background: var(--bg-secondary); border-top: 1px solid var(--border); font-size: 11px; color: var(--text-muted); flex-shrink: 0; }
.footer-left, .footer-right { display: flex; align-items: center; gap: 8px; }
.footer-progress { width: 120px; height: 4px; background: var(--border); border-radius: 2px; overflow: hidden; }
.footer-bar { height: 100%; background: var(--accent); transition: width 0.3s; }

.ctx-backdrop { position: fixed; inset: 0; z-index: 100; }
.context-menu { position: fixed; z-index: 101; background: var(--bg-card); border: 1px solid var(--border); box-shadow: var(--shadow-lg); border-radius: var(--radius-sm); min-width: 160px; padding: 4px 0; overflow: hidden; }
@keyframes fadeIn { from { opacity: 0; transform: translateY(-4px); } to { opacity: 1; transform: none; } }
.context-menu { animation: fadeIn 0.12s ease; }
.ctx-item { display: block; width: 100%; text-align: left; padding: 6px 16px; cursor: pointer; font-size: 12px; color: var(--text-primary); transition: background 0.1s; background: transparent; border: none; border-radius: 0; }
.ctx-item:hover { background: var(--bg-tertiary); }
.ctx-item.danger { color: var(--danger); }
.ctx-sep { height: 1px; background: var(--border); margin: 4px 0; }
</style>
