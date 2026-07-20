# Focus Task — Windows 桌面版打包指南

## 📦 本目录包含什么

这是对 [Focus Task](https://github.com/vipuncle2026/focus-task) 项目的 Windows 适配补丁包，包含：

| 文件 | 用途 |
|------|------|
| `frontend/src-tauri/src/lib.rs` | 修改后的 Rust 代码，新增 Windows 平台支持（通知、文件保存、窗口行为） |
| `frontend/src-tauri/tauri.conf.json` | 更新后的 Tauri 配置，添加 Windows NSIS 安装包选项 |
| `scripts/package-win.bat` | Windows 本地一键打包脚本 |
| `.github/workflows/build-windows.yml` | GitHub Actions 自动化构建工作流 |

## 🚀 两种方式打包 Windows .exe

### 方式一：GitHub Actions（推荐，零配置）

最简单的方式，无需本地安装任何工具：

1. **Fork 原项目**：到 https://github.com/vipuncle2026/focus-task 点击 Fork
2. **应用补丁**：将本目录中的修改文件覆盖到你的 Fork 仓库对应位置
3. **手动触发构建**：
   - 进入 Fork 仓库 → Actions → "Build Windows Desktop App"
   - 点击 "Run workflow" → "Run workflow"
4. **下载产物**：构建完成后在 Actions 页面下载 `Focus-Task-Windows-x64.zip`

### 方式二：本地打包（需要 Windows 环境）

#### 前置条件

| 工具 | 版本要求 | 安装方式 |
|------|---------|---------|
| Python | 3.10+ | https://www.python.org/downloads/ |
| Node.js | 18+ | https://nodejs.org/ |
| Rust | latest stable | https://rustup.rs |
| Visual Studio Build Tools | 含"C++ 桌面开发" | https://visualstudio.microsoft.com/visual-cpp-build-tools/ |

> Win10/11 通常已自带 WebView2 运行时，无需额外安装。

#### 打包步骤

```
1. 克隆项目（或下载 zip 解压）
2. 将本补丁包中的文件覆盖到对应位置
3. 双击运行 scripts/package-win.bat
4. 等待打包完成（约 5-10 分钟）
5. 在 frontend/src-tauri/target/release/bundle/nsis/ 找到安装包
```

## 🔧 修改内容详解

### lib.rs 主要改动

1. **后端二进制名称适配**
   - Windows 下自动使用 `focus-task-backend.exe`
   - macOS/Linux 继续使用 `focus-task-backend`

2. **隐藏后端控制台窗口**
   - Windows 下启动后端时使用 `CREATE_NO_WINDOW` 标志，避免弹出黑色命令行窗口

3. **原生通知支持**
   - macOS: 使用 AppleScript（原有）
   - Windows: 使用 PowerShell 调用 Windows 运行时 Toast 通知

4. **文件保存对话框**
   - macOS: 使用 AppleScript（原有）
   - Windows: 使用 PowerShell + WinForms SaveFileDialog

5. **窗口关闭行为**
   - macOS: 关闭窗口时隐藏到菜单栏（原有）
   - Windows: 关闭窗口时最小化到任务栏（标准 Windows 行为）

### tauri.conf.json 改动

- 添加 `bundle.windows.nsis` 配置，设置当前用户安装模式
- 支持中文界面，无需语言选择

## 📁 打包产物说明

打包完成后会生成：

```
frontend/src-tauri/target/release/
├── Focus Task.exe          ← 主程序（需要 backend/ 目录一起才能运行）
├── backend/                ← 后端文件（PyInstaller 打包的 FastAPI）
│   ├── focus-task-backend.exe
│   ├── alembic/
│   └── ...
└── bundle/nsis/
    └── Focus Task_0.1.0_x64-setup.exe  ← NSIS 安装包（推荐分发）
```

## ⚠️ 已知限制

1. **首次启动较慢**：PyInstaller 打包的后端首次运行需要解压临时文件，约需 10-20 秒
2. **杀毒软件误报**：PyInstaller 打包的 exe 可能被部分杀毒软件误报，建议添加到白名单
3. **安装包体积**：包含 Python 运行时 + 前端 + 后端，预计 80-150MB
4. **数据位置**：用户数据（数据库）存储在 `%APPDATA%/com.focustask.desktop/`

## 🔍 验证打包结果

打包完成后，运行 `Focus Task.exe`：
1. 应看到"聚焦任务"窗口打开
2. 默认管理员账号：`admin` / `admin123`
3. 在任务管理器中确认无多余命令行窗口
4. 尝试添加任务、切换状态，确认前后端通信正常

## 🆘 常见问题

### 构建失败："failed to run custom build command for tauri-plugin-opener"
确保安装了 Visual Studio Build Tools 的"C++ 桌面开发"工作负载。

### 构建失败："linker 'link.exe' not found"
确保 Visual Studio Build Tools 中的 MSVC 工具链已正确安装，或运行：
```
rustup default stable-msvc
```

### 打包后 exe 运行闪退
检查 `%APPDATA%/com.focustask.desktop/logs/` 下的日志文件，常见原因是后端启动超时或端口被占用。

### 想自己修改代码重新打包
修改代码后重新运行 `scripts/package-win.bat` 即可，它会自动重新构建所有组件。
