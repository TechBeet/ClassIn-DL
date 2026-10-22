# 贡献指南

感谢你对 ClassIn Downloader 的关注！我们欢迎各种形式的贡献。

## 📋 贡献类型

- 🐛 **Bug 报告**：遇到 bug 请开 Issue 描述问题
- 💡 **功能建议**：有新想法欢迎提 Issue 讨论
- 🔧 **代码贡献**：欢迎提交 PR
- 📝 **文档改进**：修复拼写错误、补充说明等
- 🌐 **翻译**：帮助本地化项目文档

## 🚀 快速开始

### 开发环境搭建

```bash
# 克隆仓库
git clone https://github.com/<your-username>/ClassIn-DL.git
cd ClassIn-DL

# 安装 Rust 1.88+
rustup default 1.88.0

# 安装依赖
# 由于项目是纯 Rust + Iced GUI，只需 Cargo
cd src-tauri
cargo build
```

### 运行开发版本

```bash
cargo run
```

## 📝 提交规范

### Commit Message 格式

遵循 [Conventional Commits](https://www.conventionalcommits.org/) 规范：

```
<type>(<scope>): <subject>

<body>

<footer>
```

**类型 (type)**：
- `feat`: 新功能
- `fix`: 修复 bug
- `docs`: 文档变更
- `style`: 代码格式（不影响功能）
- `refactor`: 重构
- `perf`: 性能优化
- `test`: 测试相关
- `chore`: 构建/工具链相关

**示例**：
```
feat(download): 添加多线程并发下载支持

- 实现 worker pool 模式
- 添加进度回调接口
- 支持断点续传

Closes #123
```

### Pull Request 规范

1. **Fork 仓库**并创建特性分支
2. **确保测试通过**：本地编译和运行无误
3. **更新文档**：如有新增功能，请补充 README 或注释
4. **描述变更**：PR 描述中说明改动内容和动机
5. **关联 Issue**：如果修复了某个 Issue，请在描述中引用

## 🎨 代码风格

### Rust 代码

遵循 [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) 和 [clippy](https://rust-lang.github.io/rust-clippy/) 推荐：

```bash
# 检查代码风格
cargo clippy -- -D warnings

# 格式化代码
cargo fmt
```

### 注意事项

- 所有公开 API 需要有文档注释
- 避免 `unwrap()` 在关键路径中使用，改用 `?` 或 `expect()`
- 添加必要的 `TODO` 注释标记待完善的功能

## 🐛 报告 Bug

请提供以下信息：

1. **操作系统**：Windows / macOS / Linux
2. **版本号**：软件版本及 Rust 版本
3. **复现步骤**：清晰的逐步操作说明
4. **预期行为**：期望的结果
5. **实际行为**：实际发生的情况
6. **日志/截图**：如有错误日志或截图请一并提供

## 📋 开发流程

1. 在 Issue 中讨论你打算做什么
2. Fork 本仓库并创建分支 (`git checkout -b feature/amazing-feature`)
3. 编写代码和测试
4. 运行 `cargo test` 确保所有测试通过
5. 运行 `cargo clippy` 确保代码风格正确
6. 提交 PR 并等待审查

## 🤝 审核流程

- 所有 PR 需要至少一个 maintainer 批准
- CI 必须通过才能合并
- 重大变更可能需要多轮讨论

## 📜 行为准则

本项目采用 [Contributor Covenant](https://www.contributor-covenant.org/) 行为准则。
请尊重其他贡献者，保持友好和建设性的讨论氛围。

## 🙏 致谢

感谢所有贡献者！