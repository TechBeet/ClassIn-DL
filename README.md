# ClassIn视频下载工具 (Tauri 2.x 版本)

基于 Rust + Tauri 2.x 重写的 ClassIn 视频解析下载工具。

## 功能特性

- 🎯 **智能解析**：从剪贴板自动解析 ClassIn 课程请求头
- ⚡ **高速下载**：支持多线程并发下载，断点续传
- 🎨 **现代界面**：基于 Svelte 5 的响应式设计
- 🔧 **灵活配置**：可调节并发数、线程数，筛选视频质量
- 📋 **剪贴板集成**：自动读取剪贴板中的请求头信息
- 📂 **目录管理**：支持选择下载目录和打开文件夹

## 技术栈

- **后端**: Rust + Tauri 2.x
- **前端**: Svelte 5 + TypeScript + Tailwind CSS
- **包管理**: pnpm
- **目标平台**: Windows, macOS

## 项目结构

`
classin-dl/
├── src-tauri/           # Rust 后端
│   ├── src/
│   │   └── lib.rs       # Tauri commands 入口
│   ├── Cargo.toml       # Rust 依赖配置
│   └── tauri.conf.json  # Tauri 应用配置
├── src/                 # Svelte 前端
│   ├── App.svelte       # 主应用组件
│   └── main.ts          # 入口文件
├── package.json         # Node.js 依赖配置
├── svelte.config.js     # Svelte 配置
└── vite.config.ts       # Vite 构建配置
`

## 安装与运行

### 前置要求

- Rust 1.88+
- Node.js 24+
- pnpm (推荐) 或 npm

### 开发模式

`ash
# 安装依赖
cd classin-dl
pnpm install

# 启动开发服务器（同时启动 Tauri 应用）
pnpm tauri dev
`

### 生产构建

`ash
# 构建应用
pnpm tauri build

# 产物位于 src-tauri/target/release/bundle/
`

## 使用方法

1. 在 ClassIn 中复制课程请求头信息
2. 点击"📋 解析视频"按钮，自动从剪贴板读取并解析
3. 解析成功后，视频列表会显示在界面中
4. 可选择下载目录、调整并发设置
5. 点击"⬇️ 开始下载"开始下载视频
6. 下载完成后，点击"📂 打开文件夹"查看下载的文件

## API 说明

### Rust 后端命令

- get_clipboard_text() - 获取剪贴板文本
- set_clipboard_text(text: String) - 设置剪贴板文本
- parse_video_info(content: String) - 解析视频信息
- download_video(request: DownloadRequest) - 开始下载视频
- select_download_dir() - 选择下载目录
- open_download_dir(path: String) - 打开下载目录

### 前端接口

- invoke('parse_video_info', { content }) - 调用解析接口
- invoke('download_video', { request }) - 调用下载接口

## 开发说明

### 添加新功能

在 src-tauri/src/lib.rs 中添加新的 #[tauri::command] 函数，然后在 invoke_handler! 中注册。

### 修改前端

在前端 src/App.svelte 中添加或修改组件，使用 TypeScript 保证类型安全。

### 构建优化

`ash
# 生产构建会进行树摇和压缩
pnpm tauri build --release
`

## 许可

MIT License

## 致谢

- [Tauri](https://tauri.app/) - 跨平台桌面应用框架
- [Svelte](https://svelte.dev/) - 现代前端框架
- [Rust](https://www.rust-lang.org/) - 系统级编程语言
