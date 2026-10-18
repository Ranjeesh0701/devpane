use std::collections::HashMap;
use std::path::PathBuf;

use devpane_core::process::session::ProcessSession;
use devpane_core::protocol::{Request, Response, read_message, write_message};

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

fn connect_to_daemon() -> Result<devpane_core::ipc::Connection, String> {
    // 1. Try connecting directly to existing daemon
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

        if let Err(e) = spawn_result {
            return Err(format!(
                "Failed to auto-start daemon binary at {}: {}",
                path.display(),
                e
            ));
        }

        // Retry connection up to 30 times (100ms interval = 3s total)
        return devpane_core::ipc::connect_with_retry(30, 100).map_err(|e| {
            format!(
                "Started daemon at {} but could not connect after 3s: {}",
                path.display(),
                e
            )
        });
    }

    Err(
        "Devpane daemon is not running and could not be found automatically.\n\nPlease start it using 'devpane up' or install the CLI.".to_string(),
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_sessions,
            start_session,
            stop_session,
            get_session_logs
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
