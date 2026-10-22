# ClassIn-DL v2.0 现代化改造

## 概述

本文档说明了 ClassIn-DL 从 WPF + C# 到 Tauri 2 + Svelte 的现代化改造工作。

## 架构变化

### 原架构
- **UI**: C# WPF (Windows Presentation Foundation)
- **下载管理**: C# 异步任务
- **配置文件**: C# 序列化

### 新架构
- **UI**: Svelte 5 + Tailwind CSS (现代化 Web 前端)
- **后端**: Rust Tauri 2.x (跨平台桌面应用)
- **下载管理**: Rust 异步处理 + Tokio
- **通信**: Tauri IPC (前端与后端通信)
- **打包**: Tauri 自动打包为 Windows/macOS/Linux 可执行文件

## 文件结构

```
ClassIn-DL/
├── src-tauri/           # Tauri 后端 (Rust)
│   ├── src/
│   │   ├── main.rs      # 应用入口和 Tauri 命令
│   │   ├── parser.rs    # 视频 URL 解析
│   │   ├── downloader.rs # 下载管理
│   │   ├── models.rs    # 数据模型
│   │   ├── clipboard.rs # 剪贴板处理
│   │   └── m3u8.rs      # HLS 流处理
│   ├── Cargo.toml       # Rust 依赖配置
│   ├── tauri.conf.json  # Tauri 配置
│   └── build.rs         # Tauri 构建配置
├── src/
│   └── frontend/        # Svelte 前端
│       ├── src/
│       │   ├── App.svelte # 主应用组件
│       │   └── main.ts    # 应用入口
│       ├── package.json   # Node.js 依赖
│       ├── vite.config.ts # Vite 构建配置
│       └── tsconfig.json  # TypeScript 配置
└── README.md            # 本文档
```

## 主要变化

### Rust 后端
- 移除了 WPF 依赖，使用 Tauri 命令模型
- 所有函数改为 async 支持 Tauri IPC
- 保留了原有的解析逻辑，提高代码复用
- 添加了 Rust 的并发处理能力 (Tokio)

### Svelte 前端
- 响应式 UI，自动更新
- 与 Tauri 后端通过 IPC 通信
- 使用 Svelte 的编译时优化
- 现代化组件化设计

## 开发命令

```bash
# 前端开发
cd src/frontend
npm install
npm run dev

# Tauri 开发
cd src-tauri
cargo tauri dev

# 生产构建
cargo tauri build
```

## 功能映射

| 原功能 | 新实现 |
|--------|--------|
| 读取剪贴板 | `parser_commands_read_clipboard` |
| 解析视频链接 | `parser_commands_parse_videos` |
| 开始下载 | `download_commands_start_downloads` |
| 取消下载 | `download_commands_cancel_download` |
| 打开文件夹 | `dir_commands_open_download_dir` |
| 设置并发数 | new max_concurrent 参数 |

## 技术栈

- **Rust**: 后端逻辑、Tauri 应用框架
- **Svelte 5**: 现代化前端框架
- **TypeScript**: 类型安全
- **Tauri 2.x**: 跨平台桌面应用
- **Tokio**: 异步运行时
- **reqwest**: HTTP 客户端
- **arboard**: 剪贴板操作
- **rfd**: 文件选择对话框

## 迁移说明

1. 保持了原有的数据模型和业务逻辑
2. UI 从事件驱动改为响应式
3. 所有异步操作都使用 Tokio
4. 打包体积大幅减小
5. 性能提升，内存占用更低

## 下一步

- [ ] 创建 UI 组件库
- [ ] 添加下载历史记录
- [ ] 实现下载队列管理
- [ ] 添加视频播放预览
- [ ] 国际化支持
