# 更新日志 (Changelog)

所有重要变更都将记录在此文件中。格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)。

## 版本规范

遵循 [语义化版本 2.0.0](https://semver.org/lang/zh-CN/)：

- **MAJOR**：不兼容的 API 变更
- **MINOR**：向后兼容的功能新增
- **PATCH**：向后兼容的问题修正

## 格式说明

**新增** : 🎉 `feat:`
**修复** : 🐛 `fix:`
**变更** : 🔧 `refactor:` / `chore:`
**文档** : 📝 `docs:`
**样式** : ✨ `style:`
**测试** : ✅ `test:`
**性能** : ⚡ `perf:`
**安全** : 🔒 `security:`

---

## [未发布]

### 🎉 新增

- 添加播放列表管理功能（待实现）

### 🔧 变更

- 重构 logger 模块：添加可注入路径支持测试，移除全局状态竞态
- 重构 m3u8 模块：修复 EXTINF 解析逻辑 bug（原来在不可达分支中）
- 重构 settings 模块：暴露内部函数用于单元测试
- 重构 models 模块：手动实现 SortConfig 的 Default（ascending=true 匹配前端）

### ✅ 测试

- 新增 78 个单元测试，总测试数从 10 增至 88
- 覆盖所有模块：models, m3u8, parser, settings, logger, clipboard, downloader
- 修复并发测试竞争条件（logger 使用 Mutex 序列化）
- 改进测试隔离性（每个测试使用独立临时文件）

### 🐛 修复

- 修复 M3U8 EXTINF 解析逻辑：现在正确跳过注释行并设置当前 duration
- 修复文件名清理：移除尾部下划线
- 修复断点续传路径：使用 with_added_extension 避免扩展名冲突
- 修复 SortConfig 默认值：ascending 默认为 true（匹配前端）

### 📝 文档

- 添加 FEATURE_GAP_ANALYSIS.md：详细对比 WPF 参考项目与 Rust/Tauri 实现的差异
- 记录已知功能缺口：播放列表管理、云同步、定时下载

---

## [v1.0.0] - 2024-XX-XX

### 发布说明

ClassIn Downloader 初始版本正式发布！

### 🎉 新增

- 支持下载 ClassIn 课程视频
- 支持断点续传
- 支持多线程并发下载
- 支持进度显示和速率控制
- 图形化界面（使用 Iced GUI）
- 支持 Windows、macOS、Linux

### 🐛 修复

- 初始版本，暂无已知问题

### 🔧 变更

- 无

### 📝 文档

- 添加 README
- 添加贡献指南
- 添加行为准则

### 🔒 安全

- 无重大安全变更

---

## [v0.9.0] - 2024-XX-XX

### 发布说明

测试版本，用于内部验证

### 🎉 新增

- 基础下载功能
- 简单的命令行界面

### 🐛 修复

- 初始版本，暂无已知问题

---

## 版本历史

完整的版本历史请参阅 [GitHub Releases](../../releases)。