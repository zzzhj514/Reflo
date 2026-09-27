# Reflo

Reflo 是独立的桌面论文研究应用，首发 macOS，并保留后续 Windows 支持能力。

## 当前状态

目前仅建立第一版代码目录与空占位文件，尚未实现功能。依赖清单和构建配置仍为空，暂不能安装、构建或运行。

## 计划功能

- PDF 文献管理、分类与元数据识别。
- PDF 阅读、高亮与批注。
- 翻译、Markdown 转换与原文定位。
- 单篇论文结构树、跨论文关系与对比阅读。

## 技术方向

- 桌面与本地服务：Tauri 2 + Rust。
- 界面：React + TypeScript + Vite。
- 本地存储与检索：SQLite + FTS5。
- PDF 阅读：PDF.js。
- PDF 解析：API 优先，暂定 MinerU。
- 论文结构与关系展示：React Flow。

## 代码结构

```text
src/
  app/                    应用入口、路由与工作区组装
  features/               文献库、阅读、解析、翻译与研究功能
  shared/                 UI、IPC、接口类型与通用工具
src-tauri/
  src/
    commands/             IPC 命令入口
    application/          业务用例编排
    domain/               领域模型与规则
    infrastructure/       数据库、文件、任务及外部服务适配
  migrations/             数据库迁移
  capabilities/           桌面权限配置
tests/                    测试样本与集成测试
public/                   静态资源
```

界面通过 Tauri IPC 调用 Rust 应用服务。Rust 管理持久化与后台任务，外部解析服务通过适配器接入。本地阅读与批注不依赖远程解析完成。
