mod tui;

use std::collections::HashMap;
use std::env;
use std::io;
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};

use devpane::config::{find_config, load_config};
use devpane::error::DevpaneError;
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

fn run() -> Result<u8, DevpaneError> {
    let mut args = env::args();
    args.next();

    let action = match args.next() {
        Some(a) => a,
        None => {
            return Err(DevpaneError::InvalidArguments(
                "No action specified. Usage: devpane <command> [arguments...]".to_string(),
            ));
        }
    };

    if action == "--help" || action == "-h" {
        println!("Usage: devpane <command> [arguments...]");
        println!("Commands:");
        println!("  init [--force] [--compose]        - Auto-detect project stack & generate devpane.yaml.");
        println!("  sync [--no-deps]                  - Sync team environment (.env, dependencies, lockfile).");
        println!("  up [--profile <name>] [--sync]    - Start enabled services (optionally by profile, sync first).");
        println!("  down                              - Stop all running project services.");
        println!("  restart [service]                 - Restart all project services or a specific service.");
        println!("  rebuild <service>                 - Clean rebuild and restart a service.");
        println!("  start <name> <program> [args...] - Start a process.");
        println!("  list                              - List active sessions.");
        println!("  stop <session_id>                 - Stop a process.");
        println!("  logs <session_id>                 - Show logs for a session.");
        println!("  shutdown                          - Shutdown the daemon.");
        return Ok(0);
    }

    if action == "init" {
        let args_vec: Vec<String> = args.collect();
        let force = args_vec.iter().any(|arg| arg == "--force" || arg == "-f");
        let compose = args_vec.iter().any(|arg| arg == "--compose" || arg == "-c");
        let cwd = env::current_dir().map_err(DevpaneError::FileOperation)?;
        match devpane::config::init_config_with_opts(&cwd, force, compose) {
            Ok((path, services)) => {
                let filename = path.file_name().unwrap_or_default().to_string_lossy();
                println!("✨ Created {} with {} service(s):\n", filename, services.len());
                for (name, cmd) in services {
                    println!("  • {:<14} -> {}", name, cmd);
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

    if action == "sync" {
        let args_vec: Vec<String> = args.collect();
        let no_deps = args_vec.iter().any(|arg| arg == "--no-deps");
        let cwd = env::current_dir().map_err(DevpaneError::FileOperation)?;

        println!("🔄 Synchronizing team environment in {}...\n", cwd.display());
        match devpane::sync::sync_project(&cwd, !no_deps) {
            Ok(report) => {
                if report.env_created {
                    println!("  ✓ Created .env from .env.example");
                } else if !report.added_env_keys.is_empty() {
                    println!("  ✓ Synced {} missing key(s) from .env.example to .env:", report.added_env_keys.len());
                    for k in &report.added_env_keys {
                        println!("    • {}", k);
                    }
                } else {
                    println!("  ✓ Environment file (.env) is up to date");
                }

                if !report.dependencies_installed.is_empty() {
                    println!("\n  ✓ Installed dependencies for {} service(s):", report.dependencies_installed.len());
                    for dep in &report.dependencies_installed {
                        println!("    • {}", dep);
                    }
                }

                if report.lockfile_updated {
                    println!("\n  ✓ Updated devpane.lock with latest configuration fingerprint");
                } else {
                    println!("\n  ✓ devpane.lock is up to date");
                }

                if !report.warnings.is_empty() {
                    println!("\n⚠️ Warnings:");
                    for w in &report.warnings {
                        println!("  • {}", w);
                    }
                }

                println!("\n✨ Team environment is fully synchronized! Run 'devpane up' to start.\n");
                return Ok(0);
            }
            Err(e) => {
                eprintln!("Error during sync: {e}");
                return Ok(1);
            }
        }
    }

    let request = match action.as_str() {
        "start" => {
            let name = args.next().ok_or_else(|| {
                DevpaneError::InvalidArguments(
                    "No name specified. Usage: start <name> <program> [args...]".to_string(),
                )
            })?;

            let program = args.next().ok_or_else(|| {
                DevpaneError::InvalidArguments(
                    "No program specified. Usage: start <name> <program> [args...]".to_string(),
                )
            })?;

            Request::Start {
                name,
                program,
                args: args.collect(),
                cwd: env::current_dir().map_err(DevpaneError::FileOperation)?,
                env: HashMap::new(),
            }
        }
        "list" => {
            if args.next().is_some() {
                return Err(DevpaneError::InvalidArguments(
                    "No arguments expected for 'list'. Usage: list".to_string(),
                ));
            }

            Request::List
        }
        "stop" => {
            let id = args.next().ok_or_else(|| {
                DevpaneError::InvalidArguments(
                    "No session ID specified. Usage: stop <session_id>".to_string(),
                )
            })?;

            if args.next().is_some() {
                return Err(DevpaneError::InvalidArguments(
                    "Only one argument expected for 'stop'. Usage: stop <session_id>".to_string(),
                ));
            }

            Request::Stop { id }
        }
        "logs" => {
            let id = args.next().ok_or_else(|| {
                DevpaneError::InvalidArguments(
                    "No session ID specified. Usage: logs <session_id>".to_string(),
                )
            })?;

            let follow = match args.next().as_deref() {
                None => false,
                Some("--follow") => true,
                Some(arg) => {
                    return Err(DevpaneError::InvalidArguments(format!(
                        "Unexpected argument: {arg}. Usage: logs <session_id> [--follow]"
                    )));
                }
            };

            Request::Logs { id, follow }
        }
        "shutdown" => {
            if args.next().is_some() {
                return Err(DevpaneError::InvalidArguments(
                    "No arguments expected for 'shutdown'. Usage: shutdown".to_string(),
                ));
            }

            Request::Shutdown
        }
        "up" => {
            let mut profile: Option<String> = None;
            let mut run_sync = false;

            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--sync" | "-s" => {
                        run_sync = true;
                    }
                    "--profile" | "-p" => {
                        profile = args.next();
                        if profile.is_none() {
                            return Err(DevpaneError::InvalidArguments(
                                "Missing profile name. Usage: up [--profile <name>] [--sync]".to_string(),
                            ));
                        }
                    }
                    other if other.starts_with("--profile=") => {
                        profile = Some(other.trim_start_matches("--profile=").to_string());
                    }
                    other => {
                        return Err(DevpaneError::InvalidArguments(format!(
                            "Unexpected argument: {other}. Usage: up [--profile <name>] [--sync]"
                        )));
                    }
                }
            }

            let path = find_config()
                .map_err(|e| DevpaneError::InvalidArguments(
                    format!("Could not find devpane.yaml (or devpane.yml) in current directory.\n\nTry:\n  1. Create a devpane.yaml file (see examples/)\n  2. Run 'devpane up' from the project root\n\nOriginal error: {}", e)
                ))?
                .canonicalize()
                .map_err(DevpaneError::FileOperation)?;
            let config = load_config(&path).map_err(|e| DevpaneError::InvalidArguments(
                format!("Failed to parse config file.\n\nCheck your YAML syntax:\n  - Proper indentation\n  - All required fields present\n  - Valid service definitions\n\nOriginal error: {}", e)
            ))?;
            let project_dir = path
                .parent()
                .ok_or(DevpaneError::InvalidResponse)?
                .to_path_buf();

            if run_sync {
                println!("🔄 Running team environment sync before startup...");
                match devpane::sync::sync_project(&project_dir, true) {
                    Ok(rep) => {
                        if rep.env_created {
                            println!("  ✓ Created .env from .env.example");
                        }
                        if !rep.added_env_keys.is_empty() {
                            println!("  ✓ Added {} missing key(s) to .env", rep.added_env_keys.len());
                        }
                        if rep.lockfile_updated {
                            println!("  ✓ Updated devpane.lock");
                        }
                    }
                    Err(e) => {
                        eprintln!("⚠️ Sync warning: {e}");
                    }
                }
            } else {
                let status = devpane::sync::check_sync_status(&project_dir);
                if !status.missing_env_keys.is_empty() {
                    eprintln!("💡 Tip: {} variable(s) in .env.example are missing from .env. Run 'devpane sync' to update.\n", status.missing_env_keys.len());
                } else if !status.lockfile_in_sync && status.lockfile_exists {
                    eprintln!("💡 Tip: devpane.yaml has changed since last sync. Run 'devpane sync' to update team lockfile.\n");
                }
            }

            let target_services = config.services_for_profile(profile.as_deref());
            if target_services.is_empty() {
                if let Some(ref p) = profile {
                    eprintln!("No enabled services matched profile '{p}'. Check your devpane.yaml profiles.");
                } else {
                    eprintln!("No enabled services found in devpane.yaml.");
                }
                return Ok(1);
            }

            // Connect to daemon and send Up request
            let mut stream = connect_or_start(50, 100)?;
            let request = Request::Up {
                project_dir: project_dir.clone(),
                services: target_services
                    .into_iter()
                    .map(|(name, service)| (name.clone(), service.clone()))
                    .collect(),
            };

            write_message(&mut stream, &request).map_err(DevpaneError::Ipc)?;
            let response = read_message::<_, Response>(&mut stream).map_err(DevpaneError::Ipc)?;

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
                    let proj_dir = project_dir.clone();
                    let tui_res = tui::run(move || {
                        let sessions = fetch_sessions()?;
                        Ok(sessions
                            .into_iter()
                            .filter(|s| s.project_dir.as_deref() == Some(&proj_dir))
                            .collect())
                    });

                    // When exiting the TUI (via q, Ctrl+C, Ctrl+Q), cleanly stop all services in the project
                    if let Ok(mut stop_stream) = connect_or_start(10, 50) {
                        let down_request = Request::Down { project_dir };
                        let _ = write_message(&mut stop_stream, &down_request);
                        let _ = read_message::<_, Response>(&mut stop_stream);
                    }

                    tui_res.map_err(DevpaneError::Tui)?;
                    return Ok(0);
                }
                Response::Error { message } => {
                    return Err(DevpaneError::DaemonError(message));
                }
                other => {
                    eprintln!("DEBUG: Received unexpected response: {:?}", other);
                    return Err(DevpaneError::InvalidResponse);
                }
            }
        }
        "down" => {
            if args.next().is_some() {
                return Err(DevpaneError::InvalidArguments(
                    "No arguments expected for 'down'. Usage: down".to_string(),
                ));
            }

            let path = find_config()
                .map_err(DevpaneError::ConfigNotFound)?
                .canonicalize()
                .map_err(DevpaneError::FileOperation)?;
            let project_dir = path
                .parent()
                .ok_or(DevpaneError::InvalidResponse)?
                .to_path_buf();

            Request::Down { project_dir }
        }
        "restart" => {
            let target = args.next();
            let (project_dir, services) = match find_config() {
                Ok(path) => {
                    let p_dir = path
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| env::current_dir().unwrap_or_default());
                    let svcs = load_config(&path).ok().map(|cfg| {
                        cfg.enabled_services()
                            .map(|(n, s)| (n.clone(), s.clone()))
                            .collect()
                    });
                    (Some(p_dir), svcs)
                }
                Err(_) => (env::current_dir().ok(), None),
            };

            Request::Restart {
                name: target,
                project_dir,
                services,
            }
        }
        "rebuild" => {
            let name = args.next().ok_or_else(|| {
                DevpaneError::InvalidArguments(
                    "No service specified. Usage: rebuild <service>".to_string(),
                )
            })?;
            Request::Rebuild { name }
        }
        _ => {
            return Err(DevpaneError::InvalidArguments(format!(
                "Invalid action: {action}. Supported actions: start, list, stop, logs, shutdown, up, down, restart, rebuild."
            )));
        }
    };

    let mut stream = connect_or_start(50, 100)?;

    write_message(&mut stream, &request).map_err(DevpaneError::Ipc)?;

    let mut response = read_message::<_, Response>(&mut stream).map_err(DevpaneError::Ipc)?;

    loop {
        match response {
            Response::Start { session } => {
                if !matches!(request, Request::Start { .. }) {
                    return Err(DevpaneError::InvalidResponse);
                }
                println!("{session:?}");
                break;
            }
            Response::List { sessions } => {
                if !matches!(request, Request::List) {
                    return Err(DevpaneError::InvalidResponse);
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
                    return Err(DevpaneError::InvalidResponse);
                }

                print!("{logs}");
                use std::io::Write;
                std::io::stdout()
                    .flush()
                    .map_err(DevpaneError::FileOperation)?;

                response = read_message::<_, Response>(&mut stream).map_err(DevpaneError::Ipc)?;
            }
            Response::LogStreamEnd => {
                if !matches!(request, Request::Logs { .. }) {
                    return Err(DevpaneError::InvalidResponse);
                }
                break;
            }
            Response::Stop => {
                if !matches!(request, Request::Stop { .. }) {
                    return Err(DevpaneError::InvalidResponse);
                }
                println!("Session stopped successfully.");
                break;
            }
            Response::Shutdown => {
                if !matches!(request, Request::Shutdown) {
                    return Err(DevpaneError::InvalidResponse);
                }
                println!("Daemon shutdown successfully.");
                break;
            }
            Response::Up { results } => {
                if !matches!(request, Request::Up { .. }) {
                    return Err(DevpaneError::InvalidResponse);
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
                    return Err(DevpaneError::InvalidResponse);
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
            Response::Restart { results } => {
                if !matches!(request, Request::Restart { .. }) {
                    return Err(DevpaneError::InvalidResponse);
                }

                if results.is_empty() {
                    println!("No services were found to restart.");
                    break;
                }

                let mut failed = false;
                for result in results {
                    if let Some(session) = result.session {
                        println!(
                            "Service '{}' restarted successfully (PID: {}).",
                            result.name, session.pid
                        );
                    } else if let Some(error) = result.error {
                        failed = true;
                        eprintln!("Failed to restart service '{}': {}", result.name, error);
                    }
                }

                if failed {
                    return Ok(1);
                }
                break;
            }
            Response::Rebuild { success, error } => {
                if !matches!(request, Request::Rebuild { .. }) {
                    return Err(DevpaneError::InvalidResponse);
                }

                if success {
                    println!("Service rebuilt and restarted successfully.");
                } else if let Some(err) = error {
                    eprintln!("Rebuild failed: {}", err);
                    return Ok(1);
                }
                break;
            }
            Response::Sync { report } => {
                if report.env_created {
                    println!("✓ Created .env from .env.example");
                }
                if !report.added_env_keys.is_empty() {
                    println!("✓ Added {} missing key(s) to .env", report.added_env_keys.len());
                }
                if report.lockfile_updated {
                    println!("✓ Updated devpane.lock");
                }
                break;
            }
            Response::Logs { .. } => {
                return Err(DevpaneError::InvalidResponse);
            }
            Response::Error { message } => {
                return Err(DevpaneError::DaemonError(message));
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

fn connect_or_start(retries: u32, delay_ms: u64) -> Result<Connection, DevpaneError> {
    match ipc::connect() {
        Ok(stream) => return Ok(stream),
        Err(_) => {
            let daemon_path = get_daemon_path().map_err(DevpaneError::FileOperation)?;

            if !daemon_path.exists() {
                return Err(DevpaneError::InvalidArguments(
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
                .map_err(|e| DevpaneError::InvalidArguments(
                    format!("Failed to start daemon.\n\nError: {}\n\nTry:\n  1. Check if another daemon is already running\n  2. Check file permissions on {}", e, daemon_path.display())
                ))?;
        }
    }

    ipc::connect_with_retry(retries, delay_ms).map_err(|e| DevpaneError::InvalidArguments(
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
