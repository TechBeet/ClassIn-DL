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
| node_modules | 待检测 |

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
│   ├── icons/           # 应用图标
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
version = "1.0.0"
edition = "2021"

[dependencies]
tauri = { version = "2", features = ["api-all"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "stream"] }

[[bin]]
name = "classin-dl"
path = "src/main.rs"
```

---

## 6. 前端 package.json 依赖清单

```json
{
  "name": "classin-dl-app",
  "version": "1.0.0",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "svelte": "^5.0.0",
    "svelte-material-ui": "^4.5.0",
    "svelte-preprocess": "^5.0.0"
  },
  "devDependencies": {
    "vite": "^5.0.0",
    "@sveltejs/vite-plugin-svelte": "^5.0.0"
  }
}
```

---

## 7. tauri.conf.json 关键配置

```json
{
  "package": {
    "productName": "ClassIn-DL",
    "version": "1.0.0"
  },
  "tauri": {
    "allowlist": {
      "all": true
    },
    "bundle": {
      "active": true,
      "targets": ["msi", "dmg", "deb"],
      "identifier": "com.classin.dl"
    },
    "security": {
      "csp": "default-src 'self'; script-src 'self' 'unsafe-inline';"
    }
  }
}
```

---

## 8. Dev/Build 命令

### 开发模式
```bash
# 开发时同时运行前端和后端
npm run dev
cargo tauri dev
```

### 构建发布
```bash
# 仅构建前端
npm run build

# 构建完整应用
cargo tauri build
```

---

## 9. 从现有 Iced 代码迁移步骤

1. **分析现有 Iced 代码结构**，识别组件树
2. **在 Svelte 中实现等价组件**，使用 svelte-material-ui 库
3. **迁移状态管理逻辑**，从 Iced 状态机转为 Svelte stores
4. **测试各功能模块**，确保视频解析下载正常
5. **集成 Node.js 视频解析库**，通过 Tauri 命令层调用
6. **UI 视觉调优**，使用 CSS 匹配 WPF 样式

---

## 10. 风险与回退方案

### 主要风险
- **包体较大**：Tauri + Svelte 打包后约 15-25MB，而纯 Iced 约 5-10MB
- **性能**：Web 视图渲染复杂视频列表可能有轻微卡顿
- **兼容性**：部分 Node.js 库可能需要适配

### 回退方案
- 如 Svelte 迁移困难，可回退到 Iced 并增加 CSS 组件库支持
- 考虑混合方案：核心视频处理用 Rust 原生，UI 保持现有 Iced 架构

---

## 11. 结论

**强烈推荐 Tauri2 + Svelte 方案**，理由：
1. 与现有 WPF 视觉高度一致
2. 技术成熟，社区支持好
3. 迁移成本相对较低
4. 视频解析依赖可复用

如需深入讨论或调整，请随时提出。
```