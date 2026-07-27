# Focus Task

Focus Task 是一款以四象限任务管理为核心的轻量桌面任务工具，支持本地桌面运行和浏览器 Web 访问。系统由 Vue 3 前端、Tauri 2 桌面壳和 FastAPI 后端组成。

## 核心功能

- **四象限任务管理**：按「重要/紧急」维度组织任务，支持拖拽移动、右键快捷操作
- **任务详情**：标题、备注、标签、开始/截止日期、重复规则、多类型提醒
- **多视图**：四象限视图、今日任务、已完成任务、统计报告
- **周/月总结**：核心指标环比、象限健康度评估、高光任务、改进建议
- **提醒通知**：开始时/截止时/过期后三类提醒，支持桌面原生通知
- **本地缓存与增量同步**：离线可用，恢复连接后自动同步
- **用户管理**：多用户隔离、管理员权限、密码策略

## 下载与安装

### Windows 桌面版

1. 前往 [Releases 页面](https://github.com/boater-man/focus-task/releases) 下载最新安装包
2. 双击运行安装程序，按提示完成安装
3. 启动应用即可使用

**系统要求：**
- Windows 10 1803+ 或 Windows 11
- 需要 WebView2 运行时（Win10/11 通常已预装）
- 若未预装，从[微软官网](https://developer.microsoft.com/microsoft-edge/webview2/)下载安装

**默认管理员账号：**

| 用户名 | 密码 |
|--------|------|
| admin | admin123 |

> 首次登录后请立即在「设置 → 用户管理」中修改默认密码。

### macOS 桌面版

原始项目 [vipuncle2026/focus-task](https://github.com/vipuncle2026/focus-task) 提供 macOS 版本。

## 数据存储

| 平台 | 路径 |
|------|------|
| Windows | `%APPDATA%/com.focustask.desktop/` |
| macOS | `~/Library/Application Support/com.focustask.desktop/` |

## 更新日志

### v0.1.0-windows (2026-07-27)

**首个 Windows 桌面版发布**

- 完成 Windows 全平台适配（Tauri 2 + NSIS 安装包）
- 后端使用 PyInstaller 打包，随安装包分发
- 启动时自动拉起后端服务，退出时自动关闭
- 隐藏后端控制台窗口，无黑框干扰
- 支持 Windows 原生托盘最小化
- 通过 GitHub Actions 自动化构建

### v0.1.0 (原始项目)

- 四象限任务管理核心功能
- Vue 3 前端 + FastAPI 后端 + SQLite 存储
- Tauri 2 桌面壳（macOS）
- 用户注册/登录/管理
- 增量同步与冲突处理
- 周/月总结报告
- 提醒与通知系统

## 技术栈

| 层 | 技术 |
|----|------|
| 前端 | Vue 3, TypeScript, Vite |
| 桌面壳 | Tauri 2 (Rust) |
| 后端 | Python FastAPI, SQLite, Alembic |
| 构建 | GitHub Actions (Windows) / 本地脚本 (macOS) |

## 许可证

MIT

