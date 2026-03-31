use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager, RunEvent, WebviewWindow, WindowEvent};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

const DEFAULT_PORT: u16 = 8080;
const HEALTH_TIMEOUT: Duration = Duration::from_secs(180);
const HEALTH_INTERVAL: Duration = Duration::from_millis(1200);

#[derive(Clone)]
struct BackendState {
    child: Arc<BackendChild>,
    runtime_dir: PathBuf,
    log_path: PathBuf,
    admin_url: String,
}

struct BackendChild {
    child: Mutex<Option<Child>>,
}

impl Drop for BackendChild {
    fn drop(&mut self) {
        if let Ok(child) = self.child.get_mut() {
            terminate_child_process(child);
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopStatusPayload {
    title: String,
    message: String,
    phase: String,
    label: String,
    level: String,
    runtime_dir: String,
    log_path: String,
    admin_url: String,
}

fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("桌面壳发生未捕获 panic: {panic_info}");
    }));

    tauri::Builder::default()
        .setup(|app| {
            let backend = prepare_backend(app)?;
            let main_window = app
                .get_webview_window("main")
                .ok_or_else(|| anyhow("未找到主窗口"))?;

            app.manage(backend.clone());
            create_tray(app)?;
            emit_status(
                &main_window,
                "准备桌面运行环境",
                "已创建本地运行目录，正在生成默认配置并启动后端。",
                "初始化中",
                "准备启动",
                "pending",
                &backend,
            );
            start_backend_monitor(main_window, backend);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .build(tauri::generate_context!())
        .expect("构建桌面应用失败")
        .run(|app, event| {
            if let RunEvent::ExitRequested { .. } | RunEvent::Exit = event {
                cleanup_backend(app);
            }
        });
}

fn prepare_backend(app: &mut App) -> Result<BackendState, String> {
    let runtime_root = app
        .path()
        .app_local_data_dir()
        .map_err(|err| format!("解析本地数据目录失败: {err}"))?;
    let runtime_dir = runtime_root.join("server");
    let log_dir = runtime_root.join("logs");
    fs::create_dir_all(&runtime_dir).map_err(|err| format!("创建运行目录失败: {err}"))?;
    fs::create_dir_all(&log_dir).map_err(|err| format!("创建日志目录失败: {err}"))?;

    let env_path = runtime_dir.join(".env");
    if !env_path.exists() {
        let db_path = runtime_dir.join("codex2api.db");
        let env_content = load_default_env(app, &db_path)?;
        fs::write(&env_path, env_content).map_err(|err| format!("写入默认 .env 失败: {err}"))?;
    }

    let backend_binary = resolve_resource_file(app.handle(), "bin/codex2api.exe")?;
    if !backend_binary.exists() {
        return Err(format!(
            "未找到后端可执行文件: {}。请先执行 Windows 构建脚本。",
            backend_binary.display()
        ));
    }

    let port = read_port(&env_path).unwrap_or(DEFAULT_PORT);
    let admin_url = format!("http://127.0.0.1:{port}/admin/");
    let log_path = log_dir.join("codex2api-desktop.log");
    let log_file = open_log_file(&log_path)?;

    let child = spawn_backend_process(&backend_binary, &runtime_dir, log_file)?;

    Ok(BackendState {
        child: Arc::new(BackendChild {
            child: Mutex::new(Some(child)),
        }),
        runtime_dir,
        log_path,
        admin_url,
    })
}

fn create_tray(app: &mut App) -> Result<(), String> {
    let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)
        .map_err(|err| format!("创建托盘菜单失败: {err}"))?;
    let open_admin_item = MenuItem::with_id(app, "open-admin", "打开管理台", true, None::<&str>)
        .map_err(|err| format!("创建托盘菜单失败: {err}"))?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)
        .map_err(|err| format!("创建托盘菜单失败: {err}"))?;
    let menu = Menu::with_items(app, &[&show_item, &open_admin_item, &quit_item])
        .map_err(|err| format!("创建托盘菜单失败: {err}"))?;

    let tray_icon = resolve_resource_file(app.handle(), "icons/icon.png").ok();
    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Codex2API Desktop");
    if let Some(icon_path) = tray_icon {
        if let Ok(image) = tauri::image::Image::from_path(&icon_path) {
            builder = builder.icon(image);
        } else {
            eprintln!("加载托盘图标失败: {}", icon_path.display());
        }
    } else {
        eprintln!("未找到托盘图标资源，将使用系统默认图标");
    }

    builder
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                let _ = reveal_main_window(app);
            }
            "open-admin" => {
                if let Err(err) = reveal_main_window(app) {
                    eprintln!("显示窗口失败: {err}");
                    return;
                }
                if let Some(state) = app.try_state::<BackendState>() {
                    if let Some(window) = app.get_webview_window("main") {
                        navigate_to_admin(&window, &state.admin_url);
                    }
                }
            }
            "quit" => {
                cleanup_backend(app);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = reveal_main_window(tray.app_handle());
            }
        })
        .build(app)
        .map_err(|err| format!("创建托盘图标失败: {err}"))?;

    Ok(())
}

fn reveal_main_window<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| anyhow("未找到主窗口"))?;
    window
        .show()
        .map_err(|err| format!("显示窗口失败: {err}"))?;
    window
        .set_focus()
        .map_err(|err| format!("聚焦窗口失败: {err}"))?;
    Ok(())
}

fn start_backend_monitor(window: WebviewWindow, backend: BackendState) {
    thread::spawn(move || {
        emit_status(
            &window,
            "拉起本地 codex2api.exe",
            "后端进程已启动，正在等待健康检查通过。",
            "启动后端",
            "等待服务就绪",
            "pending",
            &backend,
        );

        let started_at = Instant::now();
        while started_at.elapsed() < HEALTH_TIMEOUT {
            if backend_exited(&backend) {
                emit_status(
                    &window,
                    "后端进程异常退出",
                    "codex2api.exe 在完成健康检查前已退出，请查看日志文件定位问题。",
                    "启动失败",
                    "启动失败",
                    "error",
                    &backend,
                );
                return;
            }

            if health_check(&backend.admin_url) {
                emit_status(
                    &window,
                    "服务已就绪，即将进入管理后台",
                    "桌面壳已经确认本地服务健康，正在打开内嵌管理台。",
                    "启动完成",
                    "服务已就绪",
                    "ready",
                    &backend,
                );
                thread::sleep(Duration::from_millis(500));
                navigate_to_admin(&window, &backend.admin_url);
                return;
            }

            emit_status(
                &window,
                "等待健康检查通过",
                "桌面壳正在轮询 /health；首次运行若要初始化数据库，耗时会略长一些。",
                "健康检查中",
                "等待服务响应",
                "pending",
                &backend,
            );
            thread::sleep(HEALTH_INTERVAL);
        }

        emit_status(
            &window,
            "健康检查超时",
            "桌面壳在设定时间内未等到后端就绪，请查看日志文件并确认配置是否有效。",
            "启动超时",
            "启动超时",
            "error",
            &backend,
        );
    });
}

fn emit_status(
    window: &WebviewWindow,
    title: &str,
    message: &str,
    phase: &str,
    label: &str,
    level: &str,
    backend: &BackendState,
) {
    let payload = DesktopStatusPayload {
        title: title.to_string(),
        message: message.to_string(),
        phase: phase.to_string(),
        label: label.to_string(),
        level: level.to_string(),
        runtime_dir: backend.runtime_dir.display().to_string(),
        log_path: backend.log_path.display().to_string(),
        admin_url: backend.admin_url.clone(),
    };

    if let Ok(payload_json) = serde_json::to_string(&payload) {
        let script = format!(
            "window.dispatchEvent(new CustomEvent('desktop-status', {{ detail: {payload_json} }}));"
        );
        let _ = window.eval(&script);
    }
}

fn navigate_to_admin(window: &WebviewWindow, admin_url: &str) {
    if let Ok(url_json) = serde_json::to_string(admin_url) {
        let script = format!("window.location.replace({url_json});");
        let _ = window.eval(&script);
    }
}

fn backend_exited(backend: &BackendState) -> bool {
    match backend.child.child.lock() {
        Ok(mut guard) => match guard.as_mut() {
            Some(child) => child.try_wait().ok().flatten().is_some(),
            None => true,
        },
        Err(_) => true,
    }
}

fn cleanup_backend<R: tauri::Runtime>(app: &AppHandle<R>) {
    if let Some(state) = app.try_state::<BackendState>() {
        if let Ok(mut guard) = state.child.child.lock() {
            terminate_child_process(&mut guard);
            *guard = None;
        }
    }
}

fn terminate_child_process(child: &mut Option<Child>) {
    if let Some(process) = child.as_mut() {
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(3) {
            match process.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => thread::sleep(Duration::from_millis(120)),
                Err(_) => break,
            }
        }

        let _ = process.kill();
        let _ = process.wait();
    }
}

fn spawn_backend_process(
    binary: &Path,
    runtime_dir: &Path,
    log_file: File,
) -> Result<Child, String> {
    let stderr = log_file
        .try_clone()
        .map_err(|err| format!("复制日志句柄失败: {err}"))?;
    let mut command = Command::new(binary);
    command
        .current_dir(runtime_dir)
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(stderr));

    #[cfg(target_os = "windows")]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
        .spawn()
        .map_err(|err| format!("启动 codex2api.exe 失败: {err}"))
}

fn open_log_file(path: &Path) -> Result<File, String> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("打开日志文件失败: {err}"))
}

fn load_default_env(app: &App, db_path: &Path) -> Result<String, String> {
    let template_path = resolve_resource_file(app.handle(), "resources/.env.sqlite.example")?;
    if !template_path.exists() {
        return Err(format!(
            "未找到 SQLite 模板配置文件: {}",
            template_path.display()
        ));
    }
    let template = fs::read_to_string(&template_path)
        .map_err(|err| format!("读取 SQLite 模板配置失败: {err}"))?;
    let db_value = format!("DATABASE_PATH={}", normalize_path(db_path));

    let mut lines = Vec::new();
    let mut has_db_path = false;
    for line in template.lines() {
        if line.starts_with("DATABASE_PATH=") {
            lines.push(db_value.clone());
            has_db_path = true;
        } else {
            lines.push(line.to_string());
        }
    }
    if !has_db_path {
        lines.push(db_value);
    }

    lines.push(String::new());
    lines.push(String::from(
        "# 说明：桌面版默认使用本地 SQLite 文件，必要时可自行修改为其他配置。",
    ));
    Ok(lines.join("\n"))
}

fn normalize_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn read_port(env_path: &Path) -> Option<u16> {
    let env_content = fs::read_to_string(env_path).ok()?;
    env_content.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || !trimmed.starts_with("CODEX_PORT=") {
            return None;
        }
        trimmed
            .split_once('=')
            .and_then(|(_, value)| value.trim().parse::<u16>().ok())
    })
}

fn health_check(admin_url: &str) -> bool {
    let base = admin_url.trim_end_matches("/admin/");
    let host_port = base.trim_start_matches("http://");
    let addr = match host_port.to_socket_addrs() {
        Ok(mut addrs) => match addrs.next() {
            Some(addr) => addr,
            None => return false,
        },
        Err(_) => return false,
    };

    let mut stream = match TcpStream::connect_timeout(&addr, Duration::from_secs(1)) {
        Ok(stream) => stream,
        Err(_) => return false,
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
    let request = format!("GET /health HTTP/1.1\r\nHost: {host_port}\r\nConnection: close\r\n\r\n");
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }

    let mut response = String::new();
    if stream.read_to_string(&mut response).is_err() {
        return false;
    }

    let (headers, body) = match response.split_once("\r\n\r\n") {
        Some(parts) => parts,
        None => return false,
    };
    if !headers.starts_with("HTTP/1.1 200") && !headers.starts_with("HTTP/1.0 200") {
        return false;
    }

    let body = body.trim();
    if !body.ends_with('}') {
        return false;
    }

    match serde_json::from_str::<serde_json::Value>(body) {
        Ok(value) => value.get("status").and_then(|status| status.as_str()) == Some("ok"),
        Err(_) => false,
    }
}

fn resolve_resource_file<R: tauri::Runtime>(
    app: &AppHandle<R>,
    relative: &str,
) -> Result<PathBuf, String> {
    let path = app
        .path()
        .resolve(relative, tauri::path::BaseDirectory::Resource)
        .map_err(|err| format!("解析资源路径失败: {err}"))?;
    Ok(path)
}

fn anyhow(message: &str) -> String {
    message.to_string()
}
