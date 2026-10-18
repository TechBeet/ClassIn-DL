// 界面类型定义
interface VideoItem {
  id: number;
  name: string;
  url: string;
  size: number;
  status: string;
  progress: number;
  downloaded?: number;
  speed?: string;
  eta?: string;
}

interface ParseResult {
  videos: VideoItem[];
  course_name: string;
}

<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  // 状态定义
  let clipboardContent = '';
  let videoUrl = '';
  let outputDir = '待选择';
  let loading = false;
  let error = '';
  let downloadStatus = '';
  let progress = 0;
  
  // 视频列表
  let videos: VideoItem[] = [];
  let maxConcurrent = 3;
  let maxThreads = 4;
  let filterType = '全部';
  let logs: string[] = [];
  
  // 添加日志
  function addLog(message: string) {
    const timestamp = new Date().toLocaleTimeString();
    logs.push(`[${timestamp}] ${message}`);
    if (logs.length > 100) {
      logs.shift();
    }
  }
  
  // 格式化文件大小
  function formatSize(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }
  
  // 解析视频
  async function parseVideos() {
    loading = true;
    error = '';
    addLog('开始解析剪贴板内容...');
    
    try {
      const content = await invoke('get_clipboard_text');
      if (!content) {
        throw new Error('剪贴板为空，请先复制ClassIn请求头');
      }
      
      const result = await invoke('parse_video_info', { content });
      videos = result.videos;
      addLog(`解析成功，找到 ${videos.length} 个视频`);
      addLog(`课程名称: ${result.course_name}`);
      
    } catch (e) {
      error = `解析失败: ${e}`;
      addLog(`解析失败: ${e}`);
    } finally {
      loading = false;
    }
  }
  
  // 选择下载目录
  async function selectOutputDir() {
    try {
      const dir = await invoke('select_download_dir');
      outputDir = dir;
      addLog(`选择下载目录: ${dir}`);
    } catch (e) {
      error = `选择目录失败: ${e}`;
    }
  }
  
  // 打开下载目录
  async function openOutputDir() {
    try {
      await invoke('open_download_dir', { path: outputDir });
    } catch (e) {
      addLog(`打开目录失败: ${e}`);
    }
  }
  
  // 开始下载
  async function startDownload() {
    if (videos.length === 0) {
      error = '请先解析视频';
      return;
    }
    
    loading = true;
    downloadStatus = '开始下载...';
    addLog(`开始下载 ${videos.length} 个视频，并发数: ${maxConcurrent}，线程数: ${maxThreads}`);
    
    try {
      await invoke('download_video', {
        request: {
          videos,
          output_dir: outputDir,
          max_concurrent: maxConcurrent,
          max_threads: maxThreads
        }
      });
      downloadStatus = '下载完成！';
      addLog('所有视频下载完成');
    } catch (e) {
      error = `下载失败: ${e}`;
      addLog(`下载失败: ${e}`);
    } finally {
      loading = false;
    }
  }
  
  // 过滤视频
  function getFilteredVideos() {
    if (filterType === '全部') return videos;
    return videos.filter(v => {
      const sizeMB = v.size / (1024 * 1024);
      if (filterType === '1080p以上') return sizeMB > 100;
      if (filterType === '1080p以下') return sizeMB <= 100;
      return true;
    });
  }
</script>

<div class="app-container">
  <!-- 顶部控制区 -->
  <header class="header">
    <h1>🎬 ClassIn视频下载工具</h1>
    <div class="controls">
      <button class="btn btn-primary" on:click={parseVideos} disabled={loading}>
        {loading ? '解析中...' : '📋 解析视频'}
      </button>
      <button class="btn btn-secondary" on:click={selectOutputDir}>
        📁 更改路径
      </button>
      <button class="btn btn-success" on:click={startDownload} disabled={loading || videos.length === 0}>
        {loading ? '下载中...' : '⬇️ 开始下载'}
      </button>
      <button class="btn btn-info" on:click={openOutputDir}>
        📂 打开文件夹
      </button>
    </div>
  </header>
  
  <!-- 设置区 -->
  <section class="settings">
    <div class="setting-item">
      <label>最大并发下载数:</label>
      <input type="range" min="1" max="10" value={maxConcurrent} 
             on:input={(e) => maxConcurrent = parseInt(e.currentTarget.value)}>
      <span>{maxConcurrent}</span>
    </div>
    <div class="setting-item">
      <label>最大下载线程数:</label>
      <input type="range" min="1" max="8" value={maxThreads} 
             on:input={(e) => maxThreads = parseInt(e.currentTarget.value)}>
      <span>{maxThreads}</span>
    </div>
    <div class="setting-item">
      <label>筛选:</label>
      <select on:change={(e) => filterType = e.currentTarget.value}>
        <option value="全部">全部</option>
        <option value="1080p以上">1080p以上</option>
        <option value="1080p以下">1080p以下</option>
      </select>
    </div>
  </section>
  
  <!-- 下载目录显示 -->
  <div class="path-display">
    <span>📂 下载目录: </span>
    <span class="path-text">{outputDir}</span>
  </div>
  
  <!-- 错误提示 -->
  {#if error}
    <div class="error-box">
      <span>❌ {error}</span>
      <button on:click={() => error = ''}>×</button>
    </div>
  {/if}
  
  <!-- 下载状态 -->
  {#if downloadStatus}
    <div class="status-box">
      {downloadStatus}
      {#if progress > 0 && progress < 100}
        <div class="progress-bar">
          <div class="progress-fill" style="width: {progress}%"></div>
        </div>
      {/if}
    </div>
  {/if}
  
  <!-- 视频列表 -->
  <section class="video-list">
    <div class="list-header">
      <span>序号</span>
      <span>视频名称</span>
      <span>大小</span>
      <span>状态</span>
      <span>进度</span>
    </div>
    {#each getFilteredVideos() as video (video.id)}
      <div class="video-item">
        <span class="index">{video.id.toString().padStart(2, '0')}</span>
        <span class="name">{video.name}</span>
        <span class="size">{formatSize(video.size)}</span>
        <span class="status">{video.status}</span>
        <div class="progress">
          <div class="progress-bar">
            <div class="progress-fill" style="width: {video.progress}%"></div>
          </div>
          <span class="progress-text">{video.progress.toFixed(1)}%</span>
        </div>
      </div>
    {/each}
  </section>
  
  <!-- 日志面板 -->
  <section class="log-panel">
    <div class="log-header">
      <span>📝 日志</span>
      <button on:click={() => logs = []}>清空</button>
    </div>
    <div class="log-content">
      {#each logs as log}
        <div class="log-line">{log}</div>
      {:else}
        <div class="log-empty">暂无日志</div>
      {/each}
    </div>
  </section>
  
  <!-- 底部状态栏 -->
  <footer class="status-bar">
    <span>已解析: {videos.length} 个视频</span>
    <span>下载目录: {outputDir}</span>
  </footer>
</div>

<style>
  * {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }
  
  .app-container {
    max-width: 1280px;
    margin: 0 auto;
    padding: 20px;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: #f5f5f5;
    min-height: 100vh;
  }
  
  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px;
    background: white;
    border-radius: 8px;
    margin-bottom: 20px;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
  }
  
  .header h1 {
    font-size: 24px;
    color: #333;
  }
  
  .controls {
    display: flex;
    gap: 10px;
  }
  
  .btn {
    padding: 10px 20px;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
    transition: all 0.2s;
  }
  
  .btn:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 2px 8px rgba(0,0,0,0.15);
  }
  
  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  
  .btn-primary { background: #2196F3; color: white; }
  .btn-secondary { background: #FF9800; color: white; }
  .btn-success { background: #4CAF50; color: white; }
  .btn-info { background: #9C27B0; color: white; }
  
  .settings {
    display: flex;
    gap: 20px;
    padding: 15px 20px;
    background: white;
    border-radius: 8px;
    margin-bottom: 15px;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
  }
  
  .setting-item {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  
  .setting-item label {
    font-weight: 500;
    color: #555;
  }
  
  .setting-item input[type="range"] {
    width: 100px;
  }
  
  .setting-item span {
    min-width: 20px;
    text-align: center;
  }
  
  .setting-item select {
    padding: 5px 10px;
    border: 1px solid #ddd;
    border-radius: 4px;
  }
  
  .path-display {
    padding: 10px 20px;
    background: white;
    border-radius: 8px;
    margin-bottom: 15px;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
  }
  
  .path-text {
    color: #666;
    font-family: monospace;
  }
  
  .error-box {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 20px;
    background: #ffebee;
    border: 1px solid #ef9a9a;
    border-radius: 8px;
    margin-bottom: 15px;
    color: #c62828;
  }
  
  .error-box button {
    background: none;
    border: none;
    font-size: 20px;
    cursor: pointer;
    color: #c62828;
  }
  
  .status-box {
    padding: 12px 20px;
    background: #e8f5e9;
    border: 1px solid #a5d6a7;
    border-radius: 8px;
    margin-bottom: 15px;
    color: #2e7d32;
  }
  
  .progress-bar {
    width: 100%;
    height: 8px;
    background: #e0e0e0;
    border-radius: 4px;
    overflow: hidden;
    margin-top: 8px;
  }
  
  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, #4CAF50, #8BC34A);
    transition: width 0.3s ease;
  }
  
  .video-list {
    background: white;
    border-radius: 8px;
    margin-bottom: 15px;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    overflow: hidden;
  }
  
  .list-header {
    display: flex;
    align-items: center;
    padding: 12px 20px;
    background: #f5f5f5;
    border-bottom: 1px solid #e0e0e0;
    font-weight: 600;
    color: #555;
  }
  
  .list-header > * {
    flex: 1;
  }
  
  .video-item {
    display: flex;
    align-items: center;
    padding: 12px 20px;
    border-bottom: 1px solid #f0f0f0;
    transition: background 0.2s;
  }
  
  .video-item:hover {
    background: #fafafa;
  }
  
  .video-item > * {
    flex: 1;
  }
  
  .index {
    width: 40px;
    text-align: center;
    color: #888;
  }
  
  .name {
    font-weight: 500;
    color: #333;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  
  .size {
    color: #666;
    text-align: right;
  }
  
  .status {
    color: #2196F3;
    text-align: center;
  }
  
  .progress {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  
  .progress-text {
    min-width: 50px;
    text-align: right;
    color: #888;
    font-size: 12px;
  }
  
  .log-panel {
    background: white;
    border-radius: 8px;
    margin-bottom: 15px;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    overflow: hidden;
  }
  
  .log-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 20px;
    background: #f5f5f5;
    border-bottom: 1px solid #e0e0e0;
    font-weight: 600;
  }
  
  .log-header button {
    padding: 5px 12px;
    background: #f44336;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
  }
  
  .log-content {
    max-height: 200px;
    overflow-y: auto;
    padding: 10px;
  }
  
  .log-line {
    padding: 4px 8px;
    font-family: monospace;
    font-size: 12px;
    color: #333;
    border-bottom: 1px solid #f0f0f0;
  }
  
  .log-empty {
    text-align: center;
    color: #999;
    padding: 20px;
  }
  
  .status-bar {
    display: flex;
    justify-content: space-between;
    padding: 10px 20px;
    background: white;
    border-radius: 8px;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    font-size: 14px;
    color: #666;
  }
</style>