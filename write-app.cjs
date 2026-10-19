const fs = require('fs');
const content = `<script>
  import { invoke } from "@tauri-apps/api/core";
  let videos = [];
  let outputDir = "Please select";
  let loading = false;
  let error = "";
  let logs = [];
  let maxConcurrent = 3;
  let maxThreads = 4;
  function addLog(msg) {
    const t = new Date().toLocaleTimeString();
    logs = [t + " " + msg, ...logs].slice(0, 100);
  }
  function formatSize(bytes) {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  }
  async function parseVideos() {
    loading = true; error = "";
    addLog("Getting clipboard...");
    try {
      const content = await invoke("get_clipboard_text");
      if (!content || content.trim() === "") throw new Error("Clipboard is empty");
      const result = await invoke("parse_video_info", { content });
      videos = result.videos;
      addLog("Found " + videos.length + " videos");
    } catch (e) { error = "Error: " + e.message; } finally { loading = false; }
  }
  async function selectDir() {
    try {
      outputDir = await invoke("select_download_dir");
      addLog("Selected: " + outputDir);
    } catch (e) { error = e.message; }
  }
  async function openDir() { await invoke("open_download_dir", { path: outputDir }); }
  async function startDownload() {
    if (videos.length === 0) { error = "Parse video first"; return; }
    loading = true; addLog("Starting download...");
    try {
      await invoke("download_video", { request: { videos, output_dir: outputDir, max_concurrent: maxConcurrent, max_threads: maxThreads } });
      addLog("Download completed!");
    } catch (e) { error = e.message; } finally { loading = false; }
  }
</script>
<div class="app">
  <div class="header">
    <h1>ClassIn Downloader</h1>
    <div class="buttons">
      <button class="btn1" onclick={parseVideos} disabled={loading}>{loading ? "Parsing..." : "Parse"}</button>
      <button class="btn2" onclick={selectDir}>Path</button>
      <button class="btn3" onclick={startDownload} disabled={loading || !videos.length}>{loading ? "Downloading..." : "Download"}</button>
      <button class="btn4" onclick={openDir}>Open</button>
    </div>
  </div>
  <div class="dir">📁 {outputDir}</div>
  {#if error}<div class="err">{error}</div>{/if}
  {#if videos.length}
    <div class="list">{#each videos as v (v.id)}<div>{v.id} - {v.name} - {formatSize(v.size)}</div>{/each}</div>
  {/if}
  <div class="logs">{logs.slice(0,50).join("\\n") || "No logs"}</div>
</div>
<style>
  .app{max-width:800px;margin:20px auto;font-family:sans-serif}
  .header{display:flex;justify-content:space-between;align-items:center;background:#fff;padding:15px;border-radius:8px;margin-bottom:10px}
  .buttons{display:flex;gap:8px}
  button{padding:8px 16px;border:none;border-radius:4px;cursor:pointer;color:#fff}
  .btn1{background:#2196F3}.btn2{background:#FF9800}.btn3{background:#4CAF50}.btn4{background:#9C27B0}
  button:disabled{opacity:0.5}
  .dir,.err,.list{background:#fff;padding:10px;border-radius:4px;margin-bottom:10px}
  .err{color:#c62828;background:#ffebee}
  .logs{background:#fff;padding:10px;border-radius:4px;font-family:monospace;font-size:12px;max-height:200px;overflow:auto}
</style>`;
fs.writeFileSync('d:/github/ClassIn-DL/src/App.svelte', content);
console.log('App.svelte written');