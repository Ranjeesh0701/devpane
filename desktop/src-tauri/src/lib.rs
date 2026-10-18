use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use devpane_core::process::session::ProcessSession;
use devpane_core::protocol::{Request, Response, read_message, write_message};

const DEVPANE_SOCKET: &str = "/tmp/devpane.sock";

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

fn connect_to_daemon() -> Result<UnixStream, String> {
    UnixStream::connect(DEVPANE_SOCKET).map_err(|error| {
        format!("Could not connect to Devpane daemon at {DEVPANE_SOCKET}. Is it running? {error}")
    })
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
