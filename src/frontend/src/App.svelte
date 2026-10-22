<script lang="ts">
import { onMount } from 'svelte';
import { invoke } from '@tauri-apps/api/core';

let videos = [];
let statusText = '就绪';

export default {};
</script>

<div class="app">
  <h1>Classin Video Downloader v2.0</h1>
  <p>状态：{statusText}</p>
  <p>视频数量：{videos.length}</p>
</div>

<style>
.app { padding: 20px; }
</style>
