use std::env;
use std::io::{self, BufRead, IsTerminal, Write};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::ipc;
use crate::protocol::{Request, Response, read_message, write_message};

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse<'a, T: Serialize> {
    jsonrpc: &'static str,
    id: &'a Value,
    result: T,
}

#[derive(Debug, Serialize)]
struct JsonRpcErrorResponse<'a> {
    jsonrpc: &'static str,
    id: &'a Value,
    error: JsonRpcError,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

fn get_daemon_path() -> io::Result<std::path::PathBuf> {
    let mut path = env::current_exe()?;
    path.pop();

    if cfg!(windows) {
        path.push("daemon.exe");
    } else {
        path.push("daemon");
    }

    Ok(path)
}

fn connect_daemon() -> Result<ipc::Connection, String> {
    if let Ok(conn) = ipc::connect() {
        return Ok(conn);
    }

    if let Ok(daemon_path) = get_daemon_path() {
        if daemon_path.exists() {
            let _ = std::process::Command::new(&daemon_path)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
    }

    ipc::connect_with_retry(30, 100).map_err(|e| {
        format!(
            "Could not connect to Devpane daemon: {e}. Please ensure Devpane is running with 'devpane up'."
        )
    })
}

fn get_sessions() -> Result<Vec<crate::process::session::ProcessSession>, String> {
    let mut conn = connect_daemon()?;
    write_message(&mut conn, &Request::List).map_err(|e| e.to_string())?;
    match read_message::<_, Response>(&mut conn).map_err(|e| e.to_string())? {
        Response::List { sessions } => Ok(sessions),
        Response::Error { message } => Err(message),
        other => Err(format!("Unexpected response: {other:?}")),
    }
}

pub fn run_mcp_server() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    if stdin.is_terminal() {
        eprintln!("[devpane] Devpane MCP server running (stdio transport).");
        eprintln!("[devpane] Listening for JSON-RPC messages on stdin (press Ctrl+C to quit)...");
    }

    for line_res in stdin.lock().lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[mcp] Error reading line: {e}");
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[mcp] Invalid JSON-RPC request: {e}");
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": {
                        "code": -32700,
                        "message": format!("Parse error: {e}")
                    }
                });
                writeln!(stdout, "{}", serde_json::to_string(&err_resp)?)?;
                stdout.flush()?;
                continue;
            }
        };

        let id_val = req.id.clone().unwrap_or(Value::Null);

        // Notifications (no id)
        if req.id.is_none() {
            if req.method == "notifications/initialized" {
                eprintln!("[mcp] Client initialized notification received");
            }
            continue;
        }

        match req.method.as_str() {
            "initialize" => {
                let init_result = json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "devpane-mcp",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                });
                let resp = JsonRpcResponse {
                    jsonrpc: "2.0",
                    id: &id_val,
                    result: init_result,
                };
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
            }
            "ping" => {
                let resp = JsonRpcResponse {
                    jsonrpc: "2.0",
                    id: &id_val,
                    result: json!({}),
                };
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
            }
            "tools/list" => {
                let tools = get_tool_definitions();
                let resp = JsonRpcResponse {
                    jsonrpc: "2.0",
                    id: &id_val,
                    result: json!({ "tools": tools }),
                };
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
            }
            "tools/call" => {
                let tool_name = req
                    .params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let arguments = req.params.get("arguments").cloned().unwrap_or(json!({}));

                let (text_output, is_error) = handle_tool_call(tool_name, arguments);

                let resp = JsonRpcResponse {
                    jsonrpc: "2.0",
                    id: &id_val,
                    result: json!({
                        "content": [
                            {
                                "type": "text",
                                "text": text_output
                            }
                        ],
                        "isError": is_error
                    }),
                };
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
            }
            _ => {
                let resp = JsonRpcErrorResponse {
                    jsonrpc: "2.0",
                    id: &id_val,
                    error: JsonRpcError {
                        code: -32601,
                        message: format!("Method not found: {}", req.method),
                    },
                };
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
            }
        }
    }

    Ok(())
}

fn get_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "devpane_list_services",
            "description": "List all active, running, and stopped services managed by Devpane with their PID, status, CPU %, RAM usage, and ports.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "devpane_get_logs",
            "description": "Fetch live output logs for a service or session. Supports tailing and error filtering to diagnose crashes, failed builds, or runtime exceptions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "service": {
                        "type": "string",
                        "description": "Name or session ID of the service (e.g. 'backend', 'frontend', 'api')."
                    },
                    "tail_lines": {
                        "type": "integer",
                        "description": "Number of recent lines to retrieve (default: 60, max: 500)."
                    },
                    "errors_only": {
                        "type": "boolean",
                        "description": "If true, filters log output to only lines containing errors, warnings, panics, or stack traces."
                    }
                },
                "required": ["service"]
            }
        }),
        json!({
            "name": "devpane_restart_service",
            "description": "Restart a specific service managed by Devpane.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "service": {
                        "type": "string",
                        "description": "Name of the service to restart (e.g. 'backend')."
                    }
                },
                "required": ["service"]
            }
        }),
        json!({
            "name": "devpane_rebuild_service",
            "description": "Trigger a clean rebuild and restart of a service using its configured build command in devpane.yaml.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "service": {
                        "type": "string",
                        "description": "Name of the service to rebuild."
                    }
                },
                "required": ["service"]
            }
        }),
        json!({
            "name": "devpane_sync_environment",
            "description": "Check and synchronize project environment variables (.env vs .env.example) and install dependencies.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "install_deps": {
                        "type": "boolean",
                        "description": "Whether to auto-install dependencies (npm/bun/cargo/pip). Defaults to false."
                    }
                }
            }
        }),
    ]
}

fn handle_tool_call(name: &str, args: Value) -> (String, bool) {
    match name {
        "devpane_list_services" => match get_sessions() {
            Ok(sessions) => {
                if sessions.is_empty() {
                    return ("No services currently running in Devpane. Run 'devpane up' in the project directory.".to_string(), false);
                }
                let mut out = String::from(
                    "# Devpane Services\n\n| Service | Status | PID | CPU | RAM | Command |\n|---|---|---|---|---|---|\n",
                );
                for s in sessions {
                    let status_str = match s.status {
                        crate::process::session::ProcessStatus::Running => "🟢 Running",
                        crate::process::session::ProcessStatus::Exited => "🔴 Exited",
                    };
                    let cpu_str = s
                        .cpu_percent
                        .map(|c| format!("{c:.1}%"))
                        .unwrap_or_else(|| "-".to_string());
                    let ram_str = s
                        .memory_bytes
                        .map(|b| format!("{:.1} MB", b as f64 / 1_048_576.0))
                        .unwrap_or_else(|| "-".to_string());
                    let cmd_str = &s.command;
                    out.push_str(&format!(
                        "| **{}** | {} | `{}` | {} | {} | `{}` |\n",
                        s.name, status_str, s.pid, cpu_str, ram_str, cmd_str
                    ));
                }
                (out, false)
            }
            Err(e) => (format!("Error listing services: {e}"), true),
        },
        "devpane_get_logs" => {
            let service_target = args.get("service").and_then(|v| v.as_str()).unwrap_or("");
            if service_target.is_empty() {
                return ("Error: 'service' argument is required.".to_string(), true);
            }
            let tail_lines = args
                .get("tail_lines")
                .and_then(|v| v.as_u64())
                .unwrap_or(60)
                .min(500) as usize;
            let errors_only = args
                .get("errors_only")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let sessions = match get_sessions() {
                Ok(s) => s,
                Err(e) => return (format!("Failed to retrieve services: {e}"), true),
            };

            let matched = sessions
                .iter()
                .find(|s| s.name.eq_ignore_ascii_case(service_target) || s.id == service_target);
            let session_id = match matched {
                Some(s) => s.id.clone(),
                None => {
                    let available: Vec<&str> = sessions.iter().map(|s| s.name.as_str()).collect();
                    return (
                        format!(
                            "Service '{}' not found. Available services: {}",
                            service_target,
                            available.join(", ")
                        ),
                        true,
                    );
                }
            };

            let mut conn = match connect_daemon() {
                Ok(c) => c,
                Err(e) => return (e, true),
            };

            let req = Request::Logs {
                id: session_id.clone(),
                follow: false,
            };

            if let Err(e) = write_message(&mut conn, &req) {
                return (format!("Failed to request logs: {e}"), true);
            }

            let mut log_text = String::new();
            loop {
                match read_message::<_, Response>(&mut conn) {
                    Ok(Response::LogChunk { logs }) => {
                        log_text.push_str(&logs);
                    }
                    Ok(Response::LogStreamEnd) => break,
                    Ok(Response::Error { message }) => {
                        return (format!("Error streaming logs: {message}"), true);
                    }
                    Ok(_) => {}
                    Err(e) => return (format!("Failed reading logs response: {e}"), true),
                }
            }

            if log_text.is_empty() {
                return (
                    format!(
                        "Service '{}' has not produced any logs yet.",
                        service_target
                    ),
                    false,
                );
            }

            let lines: Vec<&str> = if errors_only {
                let error_keywords = [
                    "error",
                    "err",
                    "fail",
                    "fatal",
                    "panic",
                    "exception",
                    "warn",
                    "traceback",
                ];
                log_text
                    .lines()
                    .filter(|line| {
                        let lower = line.to_lowercase();
                        error_keywords.iter().any(|&k| lower.contains(k))
                    })
                    .collect()
            } else {
                log_text.lines().collect()
            };

            let start = lines.len().saturating_sub(tail_lines);
            let result_lines = &lines[start..];

            if errors_only && result_lines.is_empty() {
                return (
                    format!(
                        "No error indicators found in recent logs for '{}'.",
                        service_target
                    ),
                    false,
                );
            }

            let out = format!(
                "## Logs for '{}' (latest {} lines{}):\n```\n{}\n```",
                service_target,
                result_lines.len(),
                if errors_only { ", error-filtered" } else { "" },
                result_lines.join("\n")
            );
            (out, false)
        }
        "devpane_restart_service" => {
            let service_name = args.get("service").and_then(|v| v.as_str()).unwrap_or("");
            if service_name.is_empty() {
                return ("Error: 'service' argument is required.".to_string(), true);
            }

            let mut conn = match connect_daemon() {
                Ok(c) => c,
                Err(e) => return (e, true),
            };

            let (project_dir, services) = match crate::config::find_config() {
                Ok(path) => {
                    let p_dir = path
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| env::current_dir().unwrap_or_default());
                    let svcs = crate::config::load_config(&path).ok().map(|cfg| {
                        cfg.enabled_services()
                            .map(|(n, s)| (n.clone(), s.clone()))
                            .collect()
                    });
                    (Some(p_dir), svcs)
                }
                Err(_) => (env::current_dir().ok(), None),
            };

            let req = Request::Restart {
                name: Some(service_name.to_string()),
                project_dir,
                services,
            };

            if let Err(e) = write_message(&mut conn, &req) {
                return (format!("Failed to send restart request: {e}"), true);
            }

            match read_message::<_, Response>(&mut conn) {
                Ok(Response::Restart { results }) => {
                    if let Some(res) = results.into_iter().next() {
                        if let Some(sess) = res.session {
                            (
                                format!(
                                    "✓ Successfully restarted service '{}' (New PID: {}, Status: {:?})",
                                    res.name, sess.pid, sess.status
                                ),
                                false,
                            )
                        } else {
                            (
                                format!(
                                    "Failed to restart '{}': {}",
                                    res.name,
                                    res.error.unwrap_or_else(|| "Unknown error".to_string())
                                ),
                                true,
                            )
                        }
                    } else {
                        (
                            "Restart request completed with no result.".to_string(),
                            false,
                        )
                    }
                }
                Ok(Response::Error { message }) => (format!("Daemon error: {message}"), true),
                Ok(other) => (format!("Unexpected daemon response: {other:?}"), true),
                Err(e) => (format!("IPC error: {e}"), true),
            }
        }
        "devpane_rebuild_service" => {
            let service_name = args.get("service").and_then(|v| v.as_str()).unwrap_or("");
            if service_name.is_empty() {
                return ("Error: 'service' argument is required.".to_string(), true);
            }

            let mut conn = match connect_daemon() {
                Ok(c) => c,
                Err(e) => return (e, true),
            };

            let req = Request::Rebuild {
                name: service_name.to_string(),
            };

            if let Err(e) = write_message(&mut conn, &req) {
                return (format!("Failed to send rebuild request: {e}"), true);
            }

            match read_message::<_, Response>(&mut conn) {
                Ok(Response::Rebuild { success, error }) => {
                    if success {
                        (
                            format!(
                                "✓ Successfully rebuilt and restarted service '{}'.",
                                service_name
                            ),
                            false,
                        )
                    } else {
                        (
                            format!(
                                "Failed to rebuild service '{}': {}",
                                service_name,
                                error.unwrap_or_else(|| "Unknown error".to_string())
                            ),
                            true,
                        )
                    }
                }
                Ok(Response::Error { message }) => (format!("Daemon error: {message}"), true),
                Ok(other) => (format!("Unexpected daemon response: {other:?}"), true),
                Err(e) => (format!("IPC error: {e}"), true),
            }
        }
        "devpane_sync_environment" => {
            let install_deps = args
                .get("install_deps")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let cwd = match env::current_dir() {
                Ok(d) => d,
                Err(e) => return (format!("Failed to get current directory: {e}"), true),
            };

            match crate::sync::sync_project(&cwd, install_deps) {
                Ok(rep) => {
                    let mut msg = String::from("### Devpane Environment Sync Report\n\n");
                    if rep.env_created {
                        msg.push_str("- ✓ Created `.env` from `.env.example`\n");
                    } else if !rep.added_env_keys.is_empty() {
                        msg.push_str(&format!(
                            "- ✓ Added {} missing key(s) to `.env`: {}\n",
                            rep.added_env_keys.len(),
                            rep.added_env_keys.join(", ")
                        ));
                    } else {
                        msg.push_str("- ✓ `.env` is fully synchronized with `.env.example`\n");
                    }

                    if !rep.dependencies_installed.is_empty() {
                        msg.push_str(&format!(
                            "- ✓ Installed dependencies for: {}\n",
                            rep.dependencies_installed.join(", ")
                        ));
                    }

                    if !rep.warnings.is_empty() {
                        msg.push_str("\n**Warnings:**\n");
                        for w in rep.warnings {
                            msg.push_str(&format!("- ⚠️ {}\n", w));
                        }
                    }
                    (msg, false)
                }
                Err(e) => (format!("Sync failed: {e}"), true),
            }
        }
        _ => (format!("Unknown tool: {}", name), true),
    }
}
