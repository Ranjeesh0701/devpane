use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use devpane_core::process::session::ProcessSession;
use devpane_core::protocol::{Request, Response, read_message, write_message};

static EMBEDDED_DAEMON_RUNNING: AtomicBool = AtomicBool::new(false);
static EMBEDDED_SHUTDOWN: OnceLock<Arc<AtomicBool>> = OnceLock::new();

#[tauri::command]
fn list_sessions() -> Result<Vec<ProcessSession>, String> {
    let mut stream = connect_to_daemon()?;
    write_message(&mut stream, &Request::List)
        .map_err(|error| format!("Failed to send list request: {error}"))?;

    let response: Response = read_message(&mut stream)
        .map_err(|error| format!("Failed to read daemon response: {error}"))?;

    match response {
        Response::List { sessions } => Ok(sessions),
        Response::Error { message } => Err(message),
        other => Err(format!("Unexpected daemon response: {other:?}")),
    }
}

#[tauri::command]
fn start_session(
    name: String,
    program: String,
    args: Vec<String>,
    cwd: PathBuf,
    env: HashMap<String, String>,
) -> Result<ProcessSession, String> {
    let name = name.trim();
    let program = program.trim();

    if name.is_empty() {
        return Err("Session name cannot be empty.".to_string());
    }
    if program.is_empty() {
        return Err("Executable cannot be empty.".to_string());
    }

    let mut stream = connect_to_daemon()?;
    let request = Request::Start {
        name: name.to_string(),
        program: program.to_string(),
        args,
        cwd,
        env,
    };

    write_message(&mut stream, &request)
        .map_err(|error| format!("Failed to send start request: {error}"))?;

    let response: Response = read_message(&mut stream)
        .map_err(|error| format!("Failed to read daemon response: {error}"))?;

    match response {
        Response::Start { session } => Ok(session),
        Response::Error { message } => Err(message),
        other => Err(format!("Unexpected daemon response: {other:?}")),
    }
}

#[tauri::command]
fn stop_session(id: String) -> Result<(), String> {
    let mut stream = connect_to_daemon()?;

    write_message(&mut stream, &Request::Stop { id })
        .map_err(|error| format!("Failed to send stop request: {error}"))?;

    let response: Response = read_message(&mut stream)
        .map_err(|error| format!("Failed to read daemon response: {error}"))?;

    match response {
        Response::Stop => Ok(()),
        Response::Error { message } => Err(message),
        other => Err(format!("Unexpected daemon response: {other:?}")),
    }
}

#[tauri::command]
fn stop_project(project_dir: PathBuf) -> Result<(), String> {
    let mut stream = connect_to_daemon()?;

    write_message(&mut stream, &Request::Down { project_dir })
        .map_err(|error| format!("Failed to send down request: {error}"))?;

    let response: Response = read_message(&mut stream)
        .map_err(|error| format!("Failed to read daemon response: {error}"))?;

    match response {
        Response::Down { .. } => Ok(()),
        Response::Error { message } => Err(message),
        other => Err(format!("Unexpected daemon response: {other:?}")),
    }
}

#[tauri::command]
fn stop_all_sessions() -> Result<(), String> {
    let sessions = list_sessions()?;
    let mut stream = connect_to_daemon()?;

    for session in sessions {
        if session.status == devpane_core::process::session::ProcessStatus::Running {
            let _ = write_message(&mut stream, &Request::Stop { id: session.id.clone() });
            let _ = read_message::<_, Response>(&mut stream);
        }
    }

    Ok(())
}

#[tauri::command]
fn get_session_logs(id: String) -> Result<String, String> {
    let mut stream = connect_to_daemon()?;

    write_message(&mut stream, &Request::Logs { id, follow: false })
        .map_err(|error| format!("Failed to send logs request: {error}"))?;

    let mut logs = String::new();

    loop {
        let response: Response = read_message(&mut stream)
            .map_err(|error| format!("Failed to read logs response: {error}"))?;

        match response {
            Response::LogChunk { logs: chunk } => logs.push_str(&chunk),
            Response::LogStreamEnd => return Ok(logs),
            Response::Error { message } => return Err(message),
            other => {
                return Err(format!("Unexpected logs response: {other:?}"));
            }
        }
    }
}

use std::process::{Command, Stdio};

fn get_daemon_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    // 1. Next to current executable
    if let Ok(mut exe) = std::env::current_exe() {
        exe.pop();
        if cfg!(windows) {
            candidates.push(exe.join("daemon.exe"));
        } else {
            candidates.push(exe.join("daemon"));
            candidates.push(exe.join("../Resources/daemon"));
        }
    }

    // 2. In user's cargo bin
    if let Some(home) = dirs::home_dir() {
        if cfg!(windows) {
            candidates.push(home.join(".cargo/bin/daemon.exe"));
        } else {
            candidates.push(home.join(".cargo/bin/daemon"));
        }
    }

    // 3. In standard system paths
    if cfg!(windows) {
        candidates.push(PathBuf::from("C:\\Program Files\\devpane\\daemon.exe"));
    } else {
        candidates.push(PathBuf::from("/usr/local/bin/daemon"));
        candidates.push(PathBuf::from("/opt/homebrew/bin/daemon"));
    }

    // 4. In current workspace debug / release target (during local development)
    if cfg!(windows) {
        candidates.push(PathBuf::from("../../target/debug/daemon.exe"));
        candidates.push(PathBuf::from("../../target/release/daemon.exe"));
    } else {
        candidates.push(PathBuf::from("../../target/debug/daemon"));
        candidates.push(PathBuf::from("../../target/release/daemon"));
    }

    candidates
}

#[derive(serde::Serialize)]
struct CliStatus {
    installed: bool,
    path: Option<String>,
}

#[tauri::command]
fn get_cli_status() -> CliStatus {
    let mut check_paths = Vec::new();
    if let Some(home) = dirs::home_dir() {
        check_paths.push(home.join(".cargo/bin/devpane"));
        check_paths.push(home.join(".local/bin/devpane"));
    }
    check_paths.push(PathBuf::from("/usr/local/bin/devpane"));
    check_paths.push(PathBuf::from("/opt/homebrew/bin/devpane"));

    for path in check_paths {
        if path.is_file() {
            return CliStatus {
                installed: true,
                path: Some(path.display().to_string()),
            };
        }
    }

    if let Ok(output) = Command::new("which").arg("devpane").output() {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() {
                return CliStatus {
                    installed: true,
                    path: Some(path_str),
                };
            }
        }
    }

    CliStatus {
        installed: false,
        path: None,
    }
}

fn connect_to_daemon() -> Result<devpane_core::ipc::Connection, String> {
    // 1. Try connecting directly to existing daemon (running via CLI or previous launch)
    if let Ok(conn) = devpane_core::ipc::connect() {
        return Ok(conn);
    }

    // 2. If not running, find the daemon binary and spawn it
    let daemon_path = get_daemon_candidates()
        .into_iter()
        .find(|path| path.is_file());

    if let Some(path) = daemon_path {
        let spawn_result = Command::new(&path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn();

        if let Ok(_) = spawn_result {
            if let Ok(conn) = devpane_core::ipc::connect_with_retry(20, 100) {
                return Ok(conn);
            }
        }
    }

    // 3. Zero-click embedded daemon fallback: launch background daemon thread in-process!
    if !EMBEDDED_DAEMON_RUNNING.swap(true, Ordering::SeqCst) {
        let shutdown = Arc::new(AtomicBool::new(false));
        let _ = EMBEDDED_SHUTDOWN.set(Arc::clone(&shutdown));
        std::thread::spawn(move || {
            if let Err(e) = devpane_core::daemon::run_daemon(shutdown) {
                eprintln!("Embedded Devpane daemon error: {e}");
            }
        });
    }

    // Connect to the embedded daemon
    devpane_core::ipc::connect_with_retry(30, 100).map_err(|e| {
        format!("Failed to auto-start Devpane daemon: {e}")
    })
}

use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let toggle_item = MenuItemBuilder::with_id("toggle", "Show / Hide Devpane").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quit Devpane").build(app)?;

            let menu = MenuBuilder::new(app)
                .items(&[&toggle_item, &quit_item])
                .build()?;

            let mut tray_builder = TrayIconBuilder::with_id("devpane-tray")
                .menu(&menu)
                .tooltip("Devpane - Local Development Simplified")
                .show_menu_on_left_click(false)
                .on_menu_event(move |app_handle, event| match event.id().as_ref() {
                    "toggle" => {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                    }
                    "quit" => {
                        if let Some(shutdown) = EMBEDDED_SHUTDOWN.get() {
                            shutdown.store(true, Ordering::SeqCst);
                            let _ = devpane_core::ipc::remove_socket();
                        }
                        app_handle.exit(0);
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
                        let app_handle = tray.app_handle();
                        if let Some(window) = app_handle.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                    }
                });

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }

            let _tray = tray_builder.build(app)?;

            // Proactively warm up daemon connection on startup
            std::thread::spawn(|| {
                let _ = connect_to_daemon();
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_sessions,
            start_session,
            stop_session,
            stop_project,
            stop_all_sessions,
            get_session_logs,
            get_cli_status
        ])
        .build(tauri::generate_context!())
        .expect("error while building Tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit = event {
                if let Some(shutdown) = EMBEDDED_SHUTDOWN.get() {
                    shutdown.store(true, Ordering::SeqCst);
                    let _ = devpane_core::ipc::remove_socket();
                }
            }
        });
}
