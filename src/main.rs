mod tui;

use std::collections::HashMap;
use std::env;
use std::io;
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};

use devpane::config::{find_config, load_config};
use devpane::error::DevboxError;
use devpane::ipc;
use devpane::ipc::Connection;
use devpane::process::session::{ProcessSession, ProcessStatus};
use devpane::protocol::{Request, Response, read_message, write_message};

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<u8, DevboxError> {
    let mut args = env::args();
    args.next();

    let action = match args.next() {
        Some(a) => a,
        None => {
            return Err(DevboxError::InvalidArguments(
                "No action specified. Usage: devpane <command> [arguments...]".to_string(),
            ));
        }
    };

    if action == "--help" || action == "-h" {
        println!("Usage: devpane <command> [arguments...]");
        println!("Commands:");
        println!("  init [--force]                    - Auto-detect project stack & generate devpane.yaml.");
        println!("  up                                - Start all enabled services from devpane.yaml.");
        println!("  down                              - Stop all running project services.");
        println!("  start <name> <program> [args...] - Start a process.");
        println!("  list                              - List active sessions.");
        println!("  stop <session_id>                 - Stop a process.");
        println!("  logs <session_id>                 - Show logs for a session.");
        println!("  shutdown                          - Shutdown the daemon.");
        return Ok(0);
    }

    if action == "init" {
        let force = args.any(|arg| arg == "--force" || arg == "-f");
        let cwd = env::current_dir().map_err(DevboxError::FileOperation)?;
        match devpane::config::init_config(&cwd, force) {
            Ok((path, services)) => {
                let filename = path.file_name().unwrap_or_default().to_string_lossy();
                println!("✨ Created {} with {} service(s):\n", filename, services.len());
                for (name, cmd) in services {
                    println!("  • {:<12} -> {}", name, cmd);
                }
                println!("\nNext steps:");
                println!("  1. Review {}", filename);
                println!("  2. Run 'devpane up' to start your environment\n");
                return Ok(0);
            }
            Err(e) => {
                eprintln!("Error: {e}");
                return Ok(1);
            }
        }
    }

    let request = match action.as_str() {
        "start" => {
            let name = args.next().ok_or_else(|| {
                DevboxError::InvalidArguments(
                    "No name specified. Usage: start <name> <program> [args...]".to_string(),
                )
            })?;

            let program = args.next().ok_or_else(|| {
                DevboxError::InvalidArguments(
                    "No program specified. Usage: start <name> <program> [args...]".to_string(),
                )
            })?;

            Request::Start {
                name,
                program,
                args: args.collect(),
                cwd: env::current_dir().map_err(DevboxError::FileOperation)?,
                env: HashMap::new(),
            }
        }
        "list" => {
            if args.next().is_some() {
                return Err(DevboxError::InvalidArguments(
                    "No arguments expected for 'list'. Usage: list".to_string(),
                ));
            }

            Request::List
        }
        "stop" => {
            let id = args.next().ok_or_else(|| {
                DevboxError::InvalidArguments(
                    "No session ID specified. Usage: stop <session_id>".to_string(),
                )
            })?;

            if args.next().is_some() {
                return Err(DevboxError::InvalidArguments(
                    "Only one argument expected for 'stop'. Usage: stop <session_id>".to_string(),
                ));
            }

            Request::Stop { id }
        }
        "logs" => {
            let id = args.next().ok_or_else(|| {
                DevboxError::InvalidArguments(
                    "No session ID specified. Usage: logs <session_id>".to_string(),
                )
            })?;

            let follow = match args.next().as_deref() {
                None => false,
                Some("--follow") => true,
                Some(arg) => {
                    return Err(DevboxError::InvalidArguments(format!(
                        "Unexpected argument: {arg}. Usage: logs <session_id> [--follow]"
                    )));
                }
            };

            Request::Logs { id, follow }
        }
        "shutdown" => {
            if args.next().is_some() {
                return Err(DevboxError::InvalidArguments(
                    "No arguments expected for 'shutdown'. Usage: shutdown".to_string(),
                ));
            }

            Request::Shutdown
        }
        "up" => {
            if args.next().is_some() {
                return Err(DevboxError::InvalidArguments(
                    "No arguments expected for 'up'. Usage: up".to_string(),
                ));
            }

            let path = find_config()
                .map_err(|e| DevboxError::InvalidArguments(
                    format!("Could not find devpane.yaml (or devbox.yaml) in current directory.\n\nTry:\n  1. Create a devpane.yaml file (see examples/)\n  2. Run 'devpane up' from the project root\n\nOriginal error: {}", e)
                ))?
                .canonicalize()
                .map_err(DevboxError::FileOperation)?;
            let config = load_config(&path).map_err(|e| DevboxError::InvalidArguments(
                format!("Failed to parse config file.\n\nCheck your YAML syntax:\n  - Proper indentation\n  - All required fields present\n  - Valid service definitions\n\nOriginal error: {}", e)
            ))?;
            let project_dir = path
                .parent()
                .ok_or(DevboxError::InvalidResponse)?
                .to_path_buf();

            // Connect to daemon and send Up request
            let mut stream = connect_or_start(50, 100)?;
            let request = Request::Up {
                project_dir: project_dir.clone(),
                services: config
                    .enabled_services()
                    .map(|(name, service)| (name.clone(), service.clone()))
                    .collect(),
            };

            write_message(&mut stream, &request).map_err(DevboxError::Ipc)?;
            let response = read_message::<_, Response>(&mut stream).map_err(DevboxError::Ipc)?;

            // Check if services started successfully
            match response {
                Response::Up { results } => {
                    let mut failed = false;
                    for result in &results {
                        if let Some(error) = &result.error {
                            failed = true;
                            eprintln!("Failed to start service '{}': {}", result.name, error);
                        }
                    }

                    if failed {
                        return Ok(1);
                    }

                    // Services started successfully, launch TUI
                    drop(stream);
                    tui::run(fetch_sessions).map_err(DevboxError::Tui)?;
                    return Ok(0);
                }
                Response::Error { message } => {
                    return Err(DevboxError::DaemonError(message));
                }
                other => {
                    eprintln!("DEBUG: Received unexpected response: {:?}", other);
                    return Err(DevboxError::InvalidResponse);
                }
            }
        }
        "down" => {
            if args.next().is_some() {
                return Err(DevboxError::InvalidArguments(
                    "No arguments expected for 'down'. Usage: down".to_string(),
                ));
            }

            let path = find_config()
                .map_err(DevboxError::ConfigNotFound)?
                .canonicalize()
                .map_err(DevboxError::FileOperation)?;
            let project_dir = path
                .parent()
                .ok_or(DevboxError::InvalidResponse)?
                .to_path_buf();

            Request::Down { project_dir }
        }
        _ => {
            return Err(DevboxError::InvalidArguments(format!(
                "Invalid action: {action}. Supported actions: start, list, stop, logs, shutdown, up."
            )));
        }
    };

    let mut stream = connect_or_start(50, 100)?;

    write_message(&mut stream, &request).map_err(DevboxError::Ipc)?;

    let mut response = read_message::<_, Response>(&mut stream).map_err(DevboxError::Ipc)?;

    loop {
        match response {
            Response::Start { session } => {
                if !matches!(request, Request::Start { .. }) {
                    return Err(DevboxError::InvalidResponse);
                }
                println!("{session:?}");
                break;
            }
            Response::List { sessions } => {
                if !matches!(request, Request::List) {
                    return Err(DevboxError::InvalidResponse);
                }

                if sessions.is_empty() {
                    println!("No sessions found.");
                } else {
                    println!(
                        "{:<40} {:<20} {:<10} {:<10} COMMAND",
                        "ID", "NAME", "PID", "STATUS",
                    );
                    println!("{}", "-".repeat(100));

                    for session in sessions {
                        let status = match session.status {
                            ProcessStatus::Running => "Running",
                            ProcessStatus::Exited => "Exited",
                        };

                        println!(
                            "{:<40} {:<20} {:<10} {:<10} {}",
                            session.id, session.name, session.pid, status, session.command
                        );
                    }
                }
                break;
            }
            Response::LogChunk { logs } => {
                if !matches!(request, Request::Logs { .. }) {
                    return Err(DevboxError::InvalidResponse);
                }

                print!("{logs}");
                use std::io::Write;
                std::io::stdout()
                    .flush()
                    .map_err(DevboxError::FileOperation)?;

                response = read_message::<_, Response>(&mut stream).map_err(DevboxError::Ipc)?;
            }
            Response::LogStreamEnd => {
                if !matches!(request, Request::Logs { .. }) {
                    return Err(DevboxError::InvalidResponse);
                }
                break;
            }
            Response::Stop => {
                if !matches!(request, Request::Stop { .. }) {
                    return Err(DevboxError::InvalidResponse);
                }
                println!("Session stopped successfully.");
                break;
            }
            Response::Shutdown => {
                if !matches!(request, Request::Shutdown) {
                    return Err(DevboxError::InvalidResponse);
                }
                println!("Daemon shutdown successfully.");
                break;
            }
            Response::Up { results } => {
                if !matches!(request, Request::Up { .. }) {
                    return Err(DevboxError::InvalidResponse);
                }

                let mut failed = false;

                for result in results {
                    if let Some(session) = result.session {
                        println!(
                            "Service '{}' started successfully with session ID: {}",
                            result.name, session.id
                        );
                    } else if let Some(error) = result.error {
                        failed = true;
                        eprintln!("Failed to start service '{}': {}", result.name, error);
                    }
                }

                if failed {
                    return Ok(1);
                }

                break;
            }
            Response::Down { results } => {
                if !matches!(request, Request::Down { .. }) {
                    return Err(DevboxError::InvalidResponse);
                }

                if results.is_empty() {
                    println!("No running services found for this project.");
                    break;
                }

                let mut failed = false;

                for result in results {
                    if let Some(error) = result.error {
                        failed = true;
                        eprintln!("Failed to stop service '{}': {}", result.name, error);
                    } else {
                        println!("Service '{}' stopped successfully.", result.name);
                    }
                }

                if failed {
                    return Ok(1);
                }

                break;
            }
            Response::Logs { .. } => {
                return Err(DevboxError::InvalidResponse);
            }
            Response::Error { message } => {
                return Err(DevboxError::DaemonError(message));
            }
        }
    }

    Ok(0)
}

fn get_daemon_path() -> io::Result<PathBuf> {
    let mut path = env::current_exe()?;
    path.pop();

    if cfg!(windows) {
        path.push("daemon.exe");
    } else {
        path.push("daemon");
    }

    Ok(path)
}

fn connect_or_start(retries: u32, delay_ms: u64) -> Result<Connection, DevboxError> {
    match ipc::connect() {
        Ok(stream) => return Ok(stream),
        Err(_) => {
            let daemon_path = get_daemon_path().map_err(DevboxError::FileOperation)?;

            if !daemon_path.exists() {
                return Err(DevboxError::InvalidArguments(
                    format!(
                        "Daemon binary not found at: {}\n\nTry:\n  1. Build the project: cargo build\n  2. Ensure the daemon binary is in the same directory as devpane",
                        daemon_path.display()
                    )
                ));
            }

            Command::new(&daemon_path)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .map_err(|e| DevboxError::InvalidArguments(
                    format!("Failed to start daemon.\n\nError: {}\n\nTry:\n  1. Check if another daemon is already running\n  2. Check file permissions on {}", e, daemon_path.display())
                ))?;
        }
    }

    ipc::connect_with_retry(retries, delay_ms).map_err(|e| DevboxError::InvalidArguments(
        format!("Could not connect to daemon after {} retries.\n\nThe daemon may have failed to start.\nCheck for error messages above.\n\nOriginal error: {}", retries, e)
    ))
}

fn fetch_sessions() -> Result<Vec<ProcessSession>, String> {
    let mut stream =
        ipc::connect().map_err(|error| format!("Daemon connection failed: {error}"))?;

    write_message(&mut stream, &Request::List)
        .map_err(|error| format!("Failed to send list request: {error}"))?;

    let response: Response = read_message(&mut stream)
        .map_err(|error| format!("Failed to read daemon response: {error}"))?;

    match response {
        Response::List { sessions } => Ok(sessions),
        Response::Error { message } => Err(message),
        _ => Err("Unexpected response from daemon".to_string()),
    }
}
