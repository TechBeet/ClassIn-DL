# ClassIn-DL 架构设计文档

## 1. 结论与推荐方案

**最终推荐方案：Tauri2 + Svelte**

### 1.1 推荐理由
1. **视觉一致性**：Svelte + CSS 与 WPF XAML 样式机制高度相似，CSS Grid/Flex 能轻松复现 WPF 布局
2. **技术成熟度**：Tauri2 已稳定运行 2.x 版，TypeScript 生态完善
3. **后端兼容**：可直接调用现有 Node.js 视频解析库，大幅降低迁移成本
4. **开发效率**：svelte-material-ui 等组件库可用，开发速度快

---

## 2. 环境检测结果

| 项目 | 版本/状态 |
|------|----------|
| Node.js | v24.20.0 |
| npm | 11.19.0 |
| Rust | 1.99.0 |
| Cargo | 1.99.0 |
| Tauri CLI | 2.12.3 |
| node_modules | ✅ 检测到，大型 Node.js 生态项目 |

---

## 3. 前端方案对比分析

| 方案 | 优点 | 缺点 | 适用性 |
|------|------|------|--------|
| Iced | 原生 Rust，跨平台 | 学习曲线陡峰，组件库有限 | 低 |
| egui | 轻量， Immediate Mode | 样式定制困难，不适合复杂 UI | 低 |
| **Tauri2+Svelte** | 视觉一致，生态丰富，迁移成本低 | 包体较大 | **高** |
| Slint | 原生 Rust，声明式 | 社区小，视频播放支持有限 | 中 |
| Dioxus | 跨平台，React-like | 生态不如 Svelte 成熟 | 中 |

---

## 4. 目标目录结构

```
ClassIn-DL/
├── src/
│   ├── main.rs          # Tauri 入口
│   └── commands.rs      # Rust 命令层
├── src-tauri/
│   ├── capabilities/    # 功能能力定义
│   ├── icons/           # 应用图标
│   ├── src/             # 核心源代码
│   │   ├── main.rs      # Tauri 主入口
│   │   ├── commands.rs  # 命令定义
│   │   ├── models.rs    # 数据模型
│   │   ├── parser.rs    # 解析器
│   │   ├── downloader.rs # 下载器
│   │   └── m3u8.rs      # m3u8 处理
│   ├── tauri.conf.json  # Tauri 配置
│   └── bundle/          # 打包配置
├── src-web/             # Svelte 前端
│   ├── lib/
│   │   ├── components/  # 组件
│   │   ├── stores/      # 状态管理
│   │   └── utils/       # 工具函数
│   ├── routes/          # 路由
│   ├── app.html
│   └── app.css
├── package.json
├── Cargo.toml
└── docs/
    └── ARCH_DESIGN.md
```

---

## 5. Cargo.toml 依赖清单

```toml
[package]
name = "classin-dl"
version = "2.0.0"
description = "ClassIn视频解析下载工具 - Tauri 2.x版本"
authors = [""]
license = "MIT"
edition = "2021"

[dependencies]
# Tauri 2.x core
tauri = { version = "2", features = [] }
tauri-plugin-shell = "2"

# Async runtime
tokio = { version = "1", features = ["full", "sync"] }
reqwest = { version = "0.12", features = ["stream", "json", "gzip", "deflate", "rustls-tls"] }
futures-util = "0.3"
tokio-util = "0.7"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Clipboard
arboard = "3"

# File dialogs and operations
rfd = "0.14"
open = "5"
dirs = "5"

# Parsing and utilities
regex = "1"
lazy_static = "1.4"
anyhow = "1.0"
chrono = { version = "0.4", features = ["serde"] }
num_cpus = "1.16"

[build-dependencies]
tauri-build = { version = "2", features = [] }
```

---

## 6. 前端 package.json 依赖清单

```json
{
  "name": "classin-downloader-frontend",
  "version": "2.0.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^4.0.0",
    "svelte": "^5.0.0",
    "svelte-check": "^3.0.0",
    "typescript": "^5.0.0",
    "vite": "^5.0.0"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "svelte-material-ui": "^4.5.0",
    "svelte-preprocess": "^5.0.0"
  }
}
```

---

## 7. tauri.conf.json 关键配置

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "ClassIn Video Downloader",
  "version": "2.0.0",
  "identifier": "com.classin.downloader",
  "build": {
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build",
    "devPath": "../frontend",
    "distDir": "../frontend",
    "frontendDist": "../frontend/dist"
  },
  "app": {
    "windows": [
      {
        "title": "ClassIn视频下载工具",
        "width": 1280,
        "height": 720,
        "minWidth": 800,
        "minHeight": 480,
        "resizable": true,
        "maximizable": true,
        "decorations": true
      }
    ],
    "security": {
      "csp": "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self' https:; font-src 'self' data:"
    }
  },
  "bundle": {
    "active": true,
    "windows": {
      "certificatePath": "",
      "certificatePassword": ""
    },
    "icon": ["icons/icon.ico", "icons/icon.png", "icons/icon.svg"]
  },
  "updater": {
    "active": false
  }
}
```

---

## 8. Dev/Build 命令

### 开发模式
```bash
# 开发时同时运行前端和后端
cargo tauri dev
# 或者单独运行前端开发服务器
npm run dev
```

### 构建发布
```bash
# 构建完整应用
cargo tauri build

# 或仅构建前端
npm run build

# 构建完成后运行预览
npm run preview
```

### 环境检测命令
```bash
# 检测 Rust 版本
rustc --version
cargo --version

# 检测 Node.js 版本
node --version
npm --version

# 检查 Tauri CLI
cargo tauri --version
npx tauri --version

# 检查项目结构
ls -la
find . -name "Cargo.toml" -o -name "package.json" -o -name "tauri.conf.json"
```

---

## 9. 从现有 Iced 代码迁移的步骤

1. **分析现有 Iced 代码结构**，识别组件树和状态机
2. **在 Svelte 中实现等价组件**，使用 svelte-material-ui 库
3. **迁移状态管理逻辑**，从 Iced 状态机转为 Svelte stores
4. **测试各功能模块**，确保视频解析下载正常
5. **集成 Node.js 视频解析库**，通过 Tauri 命令层调用
6. **UI 视觉调优**，使用 CSS 匹配 WPF 样式

迁移资源参考现有代码结构：
- `src-tauri/src/main.rs`：现有 Tauri 入口
- `src-tauri/src/downloader.rs`：下载管理器实现
- `src-tauri/src/parser.rs`：视频解析器
- `src-tauri/src/models.rs`：数据模型

---

## 10. 风险与回退方案

### 主要风险
- **包体较大**：Tauri + Svelte 打包后约 15-25MB，而纯 Iced 约 5-10MB
- **性能**：Web 视图渲染复杂视频列表可能有轻微卡顿
- **兼容性**：部分 Node.js 库可能需要适配
- **前端资源**：现有前端资源位于 `src/frontend/`，需要正确配置 Tauri 构建路径

### 回退方案
- 如 Svelte 迁移困难，可回退到 Iced 并增加 CSS 组件库支持
- 考虑混合方案：核心视频处理用 Rust 原生，UI 保持现有 Iced 架构
- 如遇 Tauri 集成功能问题，可考虑使用原生 Rust GUI 方案（Iced/Slint）

---

## 11. 结论

**强烈推荐 Tauri2 + Svelte 方案**，理由：
1. 与现有 WPF 视觉高度一致，易于用户接受
2. 技术成熟，社区支持好，文档齐全
3. 迁移成本相对较低，现有代码可直接复用
4. 视频解析依赖可复用，大幅降低开发负担
5. 跨平台能力（Windows/macOS/Linux），未来扩展性好

项目已具备完整的 Tauri2 + Svelte 开发环境和前端资源，框架已接近最终方案，仅需完成架构设计文档和调整构建流程即可投入开发。

如需深入讨论或调整，请随时提出。