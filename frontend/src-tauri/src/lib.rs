use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::Manager;

#[cfg(target_os = "macos")]
use tauri::menu::{Menu, PredefinedMenuItem, Submenu};

const SERVICE_NAME: &str = "FocusTask";
const TOKEN_ACCOUNT: &str = "auth_token";
const USERNAME_ACCOUNT: &str = "auth_username";
const BACKEND_PORT: &str = "18765";
const BACKEND_RESOURCE_DIR: &str = "backend";
const BACKEND_DB_NAME: &str = "todo.db";
const BACKEND_SECRET_NAME: &str = "secret.key";

/// Backend binary name: includes .exe on Windows
#[cfg(target_os = "windows")]
const BACKEND_BINARY: &str = "focus-task-backend.exe";
#[cfg(not(target_os = "windows"))]
const BACKEND_BINARY: &str = "focus-task-backend";

static BACKEND_CHILD: OnceLock<Mutex<Option<Child>>> = OnceLock::new();

#[derive(Serialize, Deserialize)]
struct AuthState {
    token: String,
    username: String,
}

#[derive(Serialize, Deserialize)]
struct NativeNotificationPayload {
    title: String,
    body: String,
}

fn keyring_entry(account: &str) -> Result<Entry, String> {
    Entry::new(SERVICE_NAME, account).map_err(|err| err.to_string())
}

fn backend_child() -> &'static Mutex<Option<Child>> {
    BACKEND_CHILD.get_or_init(|| Mutex::new(None))
}

fn backend_port_is_open() -> bool {
    TcpStream::connect_timeout(
        &format!("127.0.0.1:{BACKEND_PORT}")
            .parse()
            .expect("valid backend address"),
        Duration::from_millis(150),
    )
    .is_ok()
}

fn backend_health_ok() -> bool {
    let addr = format!("127.0.0.1:{BACKEND_PORT}")
        .parse()
        .expect("valid backend address");

    let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(250)) else {
        return false;
    };

    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));

    let request = format!(
        "GET /api/health HTTP/1.1\r\nHost: 127.0.0.1:{BACKEND_PORT}\r\nConnection: close\r\n\r\n"
    );
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }

    let mut response = String::new();
    if stream.read_to_string(&mut response).is_err() {
        return false;
    }

    response.starts_with("HTTP/1.1 200")
        && response.contains(r#""status":"ok""#)
        && response.contains(r#""api_version":2"#)
}

fn wait_for_backend() -> bool {
    for _ in 0..150 {
        if backend_health_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    false
}

fn ensure_backend_database(resource_dir: &Path, app_data_dir: &Path) -> std::io::Result<PathBuf> {
    fs::create_dir_all(app_data_dir)?;

    let db_path = app_data_dir.join(BACKEND_DB_NAME);
    if !db_path.exists() {
        let seed_db_path = resource_dir.join("seed").join(BACKEND_DB_NAME);
        if seed_db_path.exists() {
            fs::copy(seed_db_path, &db_path)?;
        }
    }

    Ok(db_path)
}

fn ensure_backend_secret(app_data_dir: &Path) -> std::io::Result<String> {
    fs::create_dir_all(app_data_dir)?;

    let secret_path = app_data_dir.join(BACKEND_SECRET_NAME);
    if let Ok(existing) = fs::read_to_string(&secret_path) {
        let trimmed = existing.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let secret = format!("focus-task-desktop-{now}-{}", std::process::id());
    fs::write(&secret_path, &secret)?;
    Ok(secret)
}

fn copy_dir_recursive(source: &Path, target: &Path) -> std::io::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else {
            fs::copy(&source_path, &target_path)?;
            fs::set_permissions(&target_path, fs::metadata(&source_path)?.permissions())?;
        }
    }
    Ok(())
}

fn install_backend_binary(resource_dir: &Path, app_data_dir: &Path) -> std::io::Result<Option<PathBuf>> {
    let bundled_dir = resource_dir.join(BACKEND_RESOURCE_DIR);
    let bundled_path = bundled_dir.join(BACKEND_BINARY);
    if !bundled_path.exists() {
        return Ok(None);
    }

    let bin_dir = app_data_dir.join("bin");
    fs::create_dir_all(&bin_dir)?;
    let installed_dir = bin_dir.join(BACKEND_RESOURCE_DIR);
    let installed_path = installed_dir.join(BACKEND_BINARY);
    if installed_path.exists() {
        let bundled_metadata = fs::metadata(&bundled_path)?;
        let installed_metadata = fs::metadata(&installed_path)?;
        let installed_is_current = bundled_metadata.len() == installed_metadata.len()
            && installed_metadata.modified().unwrap_or(UNIX_EPOCH)
                >= bundled_metadata.modified().unwrap_or(UNIX_EPOCH);
        if installed_is_current {
            return Ok(Some(installed_path));
        }
    }

    let temporary_dir = bin_dir.join(format!("{BACKEND_RESOURCE_DIR}.new"));
    if temporary_dir.exists() {
        fs::remove_dir_all(&temporary_dir)?;
    }
    copy_dir_recursive(&bundled_dir, &temporary_dir)?;
    let temporary_path = temporary_dir.join(BACKEND_BINARY);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary_path, fs::Permissions::from_mode(0o755))?;
    }

    if installed_dir.exists() {
        fs::remove_dir_all(&installed_dir)?;
    }
    fs::rename(&temporary_dir, &installed_dir)?;
    Ok(Some(installed_path))
}

fn start_backend(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    if backend_health_ok() {
        return Ok(());
    }
    if backend_port_is_open() {
        return Err(format!(
            "127.0.0.1:{BACKEND_PORT} 已被占用，但不是 Focus Task 后端服务"
        )
        .into());
    }

    let resource_dir = app.path().resource_dir()?;
    let app_data_dir = app.path().app_data_dir()?;
    let Some(backend_path) = install_backend_binary(&resource_dir, &app_data_dir)? else {
        eprintln!(
            "Focus Task backend binary not found: {}",
            resource_dir
                .join(BACKEND_RESOURCE_DIR)
                .join(BACKEND_BINARY)
                .display()
        );
        return Ok(());
    };

    let db_path = ensure_backend_database(&resource_dir, &app_data_dir)?;
    let secret = ensure_backend_secret(&app_data_dir)?;

    let log_dir = app
        .path()
        .app_log_dir()
        .unwrap_or_else(|_| app_data_dir.join("logs"));
    fs::create_dir_all(&log_dir)?;

    let stdout = File::create(log_dir.join("backend.out.log"))?;
    let stderr = File::create(log_dir.join("backend.err.log"))?;

    // On Windows, hide the console window for the backend process
    #[cfg(target_os = "windows")]
    let child = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        Command::new(&backend_path)
            .current_dir(&app_data_dir)
            .env("FOCUS_TASK_BACKEND_PORT", BACKEND_PORT)
            .env(
                "FOCUS_TASK_DATABASE_URL",
                format!("sqlite:///{}", db_path.display()),
            )
            .env("FOCUS_TASK_SECRET_KEY", secret)
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?
    };

    #[cfg(not(target_os = "windows"))]
    let child = Command::new(&backend_path)
        .current_dir(&app_data_dir)
        .env("FOCUS_TASK_BACKEND_PORT", BACKEND_PORT)
        .env(
            "FOCUS_TASK_DATABASE_URL",
            format!("sqlite:///{}", db_path.display()),
        )
        .env("FOCUS_TASK_SECRET_KEY", secret)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()?;

    *backend_child().lock().unwrap() = Some(child);

    if !wait_for_backend() {
        stop_backend();
        return Err("Focus Task 后端启动超时".into());
    }

    Ok(())
}

fn stop_backend() {
    if let Some(mut child) = backend_child().lock().unwrap().take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[tauri::command]
fn load_auth_state() -> Result<Option<AuthState>, String> {
    let token_entry = keyring_entry(TOKEN_ACCOUNT)?;
    let username_entry = keyring_entry(USERNAME_ACCOUNT)?;

    let token = match token_entry.get_password() {
        Ok(value) => value,
        Err(keyring::Error::NoEntry) => return Ok(None),
        Err(err) => return Err(err.to_string()),
    };
    let username = match username_entry.get_password() {
        Ok(value) => value,
        Err(keyring::Error::NoEntry) => String::new(),
        Err(err) => return Err(err.to_string()),
    };

    Ok(Some(AuthState { token, username }))
}

#[tauri::command]
fn save_auth_state(state: AuthState) -> Result<(), String> {
    keyring_entry(TOKEN_ACCOUNT)?
        .set_password(&state.token)
        .map_err(|err| err.to_string())?;
    keyring_entry(USERNAME_ACCOUNT)?
        .set_password(&state.username)
        .map_err(|err| err.to_string())?;
    Ok(())
}

#[tauri::command]
fn clear_auth_state() -> Result<(), String> {
    let token_entry = keyring_entry(TOKEN_ACCOUNT)?;
    let username_entry = keyring_entry(USERNAME_ACCOUNT)?;

    if let Err(err) = token_entry.delete_credential() {
        if !matches!(err, keyring::Error::NoEntry) {
            return Err(err.to_string());
        }
    }
    if let Err(err) = username_entry.delete_credential() {
        if !matches!(err, keyring::Error::NoEntry) {
            return Err(err.to_string());
        }
    }

    Ok(())
}

#[tauri::command]
fn open_notification_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.notifications")
            .status()
            .map_err(|err| err.to_string())?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("powershell")
            .args([
                "-Command",
                "Start-Process 'ms-settings:notifications'",
            ])
            .status()
            .map_err(|err| err.to_string())?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("当前平台暂不支持直接打开通知设置".to_string())
}

#[tauri::command]
fn send_native_notification(payload: NativeNotificationPayload) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display notification {} with title {} sound name \"default\"",
            applescript_string(&payload.body),
            applescript_string(&payload.title)
        );
        Command::new("osascript")
            .arg("-e")
            .arg(script)
            .status()
            .map_err(|err| err.to_string())?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        // Use PowerShell toast notification on Windows
        let escaped_title = payload.title.replace('"', "`\"");
        let escaped_body = payload.body.replace('"', "`\"");
        let script = format!(
            r#"[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null
$template = '<toast><visual><binding template="ToastGeneric"><text>{}</text><text>{}</text></binding></visual></toast>'
$xml = New-Object Windows.Data.Xml.Dom.XmlDocument
$xml.LoadXml($template)
$toast = [Windows.UI.Notifications.ToastNotification]::new($xml)
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('Focus Task').Show($toast)"#,
            escaped_title, escaped_body
        );
        let _ = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .status();
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("当前平台暂不支持原生通知".to_string())
}

#[tauri::command]
fn save_text_file(filename: String, content: String) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "set targetFile to choose file name with prompt \"导出 Focus Task 备份\" default name {}\nPOSIX path of targetFile",
            applescript_string(&filename)
        );
        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|err| err.to_string())?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("-128") {
                return Ok(false);
            }
            return Err(stderr.trim().to_string());
        }

        let path = String::from_utf8(output.stdout)
            .map_err(|err| err.to_string())?
            .trim()
            .to_string();
        if path.is_empty() {
            return Ok(false);
        }
        fs::write(path, content).map_err(|err| err.to_string())?;
        return Ok(true);
    }

    #[cfg(target_os = "windows")]
    {
        // Use PowerShell Save-File dialog
        let escaped_filename = filename.replace('"', "`\"");
        let script = format!(
            r#"Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.SaveFileDialog
$dialog.Title = '导出 Focus Task 备份'
$dialog.FileName = '{}'
$dialog.Filter = 'JSON 文件 (*.json)|*.json|所有文件 (*.*)|*.*'
$result = $dialog.ShowDialog()
if ($result -eq 'OK') {{ $dialog.FileName }} else {{ '' }}"#,
            escaped_filename
        );
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .map_err(|err| err.to_string())?;

        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if path.is_empty() {
            return Ok(false);
        }
        fs::write(&path, content).map_err(|err| err.to_string())?;
        return Ok(true);
    }

    #[allow(unreachable_code)]
    Err("当前平台不支持原生文件保存".to_string())
}

#[cfg(target_os = "macos")]
fn applescript_string(value: &str) -> String {
    format!("{:?}", value)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            load_auth_state,
            save_auth_state,
            clear_auth_state,
            open_notification_settings,
            send_native_notification,
            save_text_file
        ])
        .setup(|app| {
            // ─── Native macOS menu ───
            #[cfg(target_os = "macos")]
            {
                let app_menu = Submenu::with_items(
                    app,
                    "Focus Task",
                    true,
                    &[
                        &PredefinedMenuItem::about(app, Some("关于 Focus Task"), None)?,
                        &PredefinedMenuItem::separator(app)?,
                        &PredefinedMenuItem::hide(app, Some("隐藏 Focus Task"))?,
                        &PredefinedMenuItem::hide_others(app, Some("隐藏其他"))?,
                        &PredefinedMenuItem::show_all(app, Some("全部显示"))?,
                        &PredefinedMenuItem::separator(app)?,
                        &PredefinedMenuItem::quit(app, Some("退出 Focus Task"))?,
                    ],
                )?;

                let file_menu = Submenu::with_items(
                    app,
                    "文件",
                    true,
                    &[&PredefinedMenuItem::close_window(app, Some("关闭窗口"))?],
                )?;

                let edit_menu = Submenu::with_items(
                    app,
                    "编辑",
                    true,
                    &[
                        &PredefinedMenuItem::undo(app, Some("撤销"))?,
                        &PredefinedMenuItem::redo(app, Some("重做"))?,
                        &PredefinedMenuItem::separator(app)?,
                        &PredefinedMenuItem::cut(app, Some("剪切"))?,
                        &PredefinedMenuItem::copy(app, Some("复制"))?,
                        &PredefinedMenuItem::paste(app, Some("粘贴"))?,
                        &PredefinedMenuItem::select_all(app, Some("全选"))?,
                    ],
                )?;

                let view_menu = Submenu::with_items(
                    app,
                    "视图",
                    true,
                    &[&PredefinedMenuItem::fullscreen(app, Some("进入全屏幕"))?],
                )?;

                let window_menu = Submenu::with_items(
                    app,
                    "窗口",
                    true,
                    &[
                        &PredefinedMenuItem::minimize(app, Some("最小化"))?,
                        &PredefinedMenuItem::fullscreen(app, Some("缩放"))?,
                        &PredefinedMenuItem::separator(app)?,
                        &PredefinedMenuItem::bring_all_to_front(app, Some("全部移到最前"))?,
                    ],
                )?;

                let help_menu = Submenu::with_items(
                    app,
                    "帮助",
                    true,
                    &[&PredefinedMenuItem::services(app, Some("服务"))?],
                )?;

                let menu = Menu::with_items(
                    app,
                    &[
                        &app_menu,
                        &file_menu,
                        &edit_menu,
                        &view_menu,
                        &window_menu,
                        &help_menu,
                    ],
                )?;
                app.set_menu(menu)?;
            }

            if let Err(err) = start_backend(app) {
                eprintln!("Failed to start Focus Task backend: {err}");
            }

            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // On macOS: hide to menu bar on close
            #[cfg(target_os = "macos")]
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            // On Windows: minimize to taskbar on close (standard Windows behavior)
            #[cfg(target_os = "windows")]
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.minimize();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| match event {
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => {
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            tauri::RunEvent::Exit => stop_backend(),
            _ => {}
        });
}
