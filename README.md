# ClassIn Video Downloader

[![Build](https://github.com/your-username/ClassIn-DL/actions/workflows/build.yml/badge.svg)](https://github.com/your-username/ClassIn-DL/actions/workflows/build.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.88+-orange.svg)](https://www.rust-lang.org/)

基于 Rust + Iced 框架的 ClassIn 课程视频解析下载工具。支持从剪贴板自动解析课程信息、多线程并发下载、断点续传等功能。

## ✨ 功能特性

- 🎯 **智能解析** - 从剪贴板自动解析 ClassIn 课程请求头
- ⚡ **高速下载** - 支持多线程并发下载，断点续传
- 🎨 **现代界面** - 基于 Iced 框架的响应式设计
- 🔧 **灵活配置** - 可调节并发数、筛选视频质量
- 📋 **剪贴板集成** - 自动读取剪贴板中的请求头信息
- 📂 **目录管理** - 支持选择下载目录和快速打开文件夹
- 🛡️ **跨平台** - 支持 Windows、macOS、Linux

## 🚀 快速开始

### 前置要求

- [Rust](https://www.rust-lang.org/tools/install) 1.88+

### 开发模式

\\\ash
# 克隆仓库
git clone https://github.com/your-username/ClassIn-DL.git
cd ClassIn-DL

# 安装依赖并构建
cd src-tauri
cargo build

# 启动应用
cargo run
\\\

### 生产构建

\\\ash
# 构建 Release 版本
cargo build --release

# 产物位于 src-tauri/target/release/
\\\

## 📖 使用方法

1. 在 ClassIn 课程中复制课程请求头信息（Ctrl+C）
2. 在应用中点击「📋 解析视频」按钮
3. 解析成功后，视频列表会显示在界面中
4. 可选择下载目录、调整并发设置
5. 点击「⬇️ 开始下载」开始下载视频
6. 下载完成后，点击「📂 打开文件夹」查看下载的文件

## 🏗️ 技术栈

| 层级 | 技术 |
|------|------|
| **后端** | Rust + Iced 0.13 |
| **包管理** | Cargo |
| **目标平台** | Windows, macOS, Linux |

## 📁 项目结构

\\\
classin-dl/
├── src-tauri/           # Rust 后端
│   ├── src/
│   │   ├── main.rs         # 主入口
│   │   ├── models.rs       # 数据模型
│   │   ├── clipboard.rs    # 剪贴板操作
│   │   ├── parser.rs       # URL 解析器
│   │   └── downloader.rs   # 下载管理器
│   ├── Cargo.toml          # Rust 依赖配置
│   └── tauri.conf.json     # Tauri 应用配置
├── .github/               # GitHub 工作流和模板
│   ├── workflows/
│   │   └── build.yml        # 构建工作流
│   └── ISSUE_TEMPLATE/    # Issue 模板
├── LICENSE               # 许可证
├── CONTRIBUTING.md       # 贡献指南
└── README.md             # 项目文档
\\\

## 🔄 版本历史

### v1.0.0 (Initial Release)

- ✅ 基础功能：剪贴板解析、视频下载
- ✅ 多线程并发下载
- ✅ 断点续传支持
- ✅ 进度实时更新
- ✅ 取消下载功能

## 🤝 贡献指南

欢迎贡献代码！请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 了解如何参与项目开发。

## 📄 许可证

本项目采用 [MIT License](LICENSE) 开源协议。

## 🙏 致谢

- [Iced](https://iced.rs/) - Rust GUI 框架
- [Rust](https://www.rust-lang.org/) - 系统级编程语言
- [arboard](https://crates.io/crates/arboard) - 剪贴板访问
- [rfd](https://crates.io/crates/rfd) - 文件对话框
