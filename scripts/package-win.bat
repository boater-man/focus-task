@echo off
setlocal enabledelayedexpansion
REM ================================================================
REM  Focus Task — Windows 桌面版打包脚本
REM  用法: 在 Windows 上双击运行，或在 cmd 中执行此脚本
REM  
REM  前置条件:
REM    1. Python 3.10+ 已安装并加入 PATH（推荐 3.14）
REM    2. Node.js 18+ 已安装并加入 PATH
REM    3. Rust (rustup) 已安装: https://rustup.rs
REM    4. WebView2 已安装（Win10/11 通常自带）
REM    5. Visual Studio Build Tools 已安装（含 C++ 桌面开发工作负载）
REM ================================================================

set "ROOT_DIR=%~dp0.."
set "BACKEND_DIR=%ROOT_DIR%\backend"
set "FRONTEND_DIR=%ROOT_DIR%\frontend"
set "TAURI_DIR=%FRONTEND_DIR%\src-tauri"

echo.
echo ========================================
echo   Focus Task Windows 打包工具
echo ========================================
echo.

REM ─── 检查前置工具 ───
echo [1/6] 检查环境...

where python >nul 2>&1
if %errorlevel% neq 0 (
    echo ✗ Python 未找到，请安装 Python 3.10+ 并加入 PATH
    pause
    exit /b 1
)
echo   ✓ Python: 
python --version

where node >nul 2>&1
if %errorlevel% neq 0 (
    echo ✗ Node.js 未找到，请安装 Node.js 18+
    pause
    exit /b 1
)
echo   ✓ Node.js:
node --version

where cargo >nul 2>&1
if %errorlevel% neq 0 (
    echo ✗ Rust/Cargo 未找到，请安装: https://rustup.rs
    pause
    exit /b 1
)
echo   ✓ Rust/Cargo:
cargo --version

REM ─── 安装后端依赖并用 PyInstaller 打包 ───
echo.
echo [2/6] 打包后端 (PyInstaller)...
cd /d "%BACKEND_DIR%"

REM 创建虚拟环境（如果不存在）
if not exist "venv" (
    echo   创建虚拟环境...
    python -m venv venv
)
call venv\Scripts\activate.bat

REM 安装依赖
pip install --quiet pyinstaller
pip install --quiet -r requirements.txt

REM PyInstaller 打包
pyinstaller ^
    --clean ^
    --noconfirm ^
    --name focus-task-backend ^
    --hidden-import passlib.handlers.bcrypt ^
    --add-data "alembic;alembic" ^
    --add-data "alembic.ini;." ^
    app_server.py

if %errorlevel% neq 0 (
    echo ✗ PyInstaller 打包失败
    pause
    exit /b 1
)

echo   ✓ 后端打包完成: backend\dist\focus-task-backend\

REM ─── 安装前端依赖并构建 ───
echo.
echo [3/6] 构建前端 (Vite)...
cd /d "%FRONTEND_DIR%"

if not exist "node_modules" (
    echo   安装前端依赖...
    call npm install
)

call npm run build
if %errorlevel% neq 0 (
    echo ✗ 前端构建失败
    pause
    exit /b 1
)
echo   ✓ 前端构建完成

REM ─── 运行前端测试（可选） ───
echo.
echo [4/6] 运行前端测试...
cd /d "%FRONTEND_DIR%"
call npm test 2>nul
if %errorlevel% neq 0 (
    echo   ⚠ 测试有警告，继续打包...
) else (
    echo   ✓ 测试通过
)

REM ─── Tauri 构建 ───
echo.
echo [5/6] 构建 Tauri 桌面应用...
cd /d "%TAURI_DIR%"

REM 确保 Windows target 已安装
rustup target add x86_64-pc-windows-msvc >nul 2>&1

REM 构建 NSIS 安装包
cargo tauri build --bundles nsis
if %errorlevel% neq 0 (
    echo ✗ Tauri 构建失败
    pause
    exit /b 1
)

REM ─── 整合后端到 Tauri 安装包 ───
echo.
echo [6/6] 整合后端到安装包...

REM 找到 Tauri 生成的目录
set "RELEASE_DIR=%TAURI_DIR%\target\release"

REM 将后端文件复制到 Tauri resources 对应位置
REM Tauri NSIS 安装包会将 resources 放到安装目录
REM 后端需要放在安装目录的 backend/ 子目录下
set "BUNDLE_BACKEND_DIR=%RELEASE_DIR%\backend"
if not exist "%BUNDLE_BACKEND_DIR%" mkdir "%BUNDLE_BACKEND_DIR%"

xcopy /E /Y /Q "%BACKEND_DIR%\dist\focus-task-backend\*" "%BUNDLE_BACKEND_DIR%\" >nul
echo   ✓ 后端已整合

REM ─── 完成 ───
echo.
echo ========================================
echo   打包完成！
echo ========================================
echo.
echo   安装包位置:
echo   %TAURI_DIR%\target\release\bundle\nsis\
echo.
echo   解压目录（免安装版）:
echo   %RELEASE_DIR%\
echo.
echo   如需创建便携版，可将 %RELEASE_DIR% 目录
echo   整体复制到目标机器运行 Focus Task.exe
echo.
pause
