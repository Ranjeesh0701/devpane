use std::io;
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

/// Check if a TCP port is currently in use on localhost.
pub fn is_port_in_use(port: u16) -> bool {
    if TcpListener::bind(("127.0.0.1", port)).is_err() {
        return true;
    }
    TcpStream::connect(("127.0.0.1", port)).is_ok()
}

/// Find PID(s) listening on a given TCP port.
pub fn find_pids_on_port(port: u16) -> Vec<u32> {
    #[cfg(unix)]
    {
        let output = Command::new("lsof")
            .args(["-ti", &format!("tcp:{}", port)])
            .output();

        if let Ok(out) = output {
            let s = String::from_utf8_lossy(&out.stdout);
            let mut pids = Vec::new();
            for line in s.lines() {
                if let Ok(pid) = line.trim().parse::<u32>() {
                    if pid > 0 && !pids.contains(&pid) {
                        pids.push(pid);
                    }
                }
            }
            return pids;
        }
    }

    #[cfg(windows)]
    {
        let output = Command::new("cmd")
            .args(["/C", &format!("netstat -ano | findstr :{}", port)])
            .output();

        if let Ok(out) = output {
            let s = String::from_utf8_lossy(&out.stdout);
            let mut pids = Vec::new();
            for line in s.lines() {
                if let Some(last_token) = line.split_whitespace().last() {
                    if let Ok(pid) = last_token.parse::<u32>() {
                        if pid > 0 && !pids.contains(&pid) {
                            pids.push(pid);
                        }
                    }
                }
            }
            return pids;
        }
    }

    Vec::new()
}

/// Kill conflicting processes occupying the port and wait for it to be freed.
pub fn free_port_if_occupied(port: u16) -> io::Result<Option<u32>> {
    if !is_port_in_use(port) {
        return Ok(None);
    }

    let pids = find_pids_on_port(port);
    let my_pid = std::process::id();

    for pid in &pids {
        if *pid == my_pid {
            continue;
        }
        println!("⚠️  Port {} is occupied by PID {}. Terminating conflicting process...", port, pid);
        let _ = crate::process::operations::stop_process(*pid);
    }

    // Wait up to 2 seconds for the port to become available
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        if !is_port_in_use(port) {
            return Ok(pids.into_iter().next());
        }
        thread::sleep(Duration::from_millis(100));
    }

    Ok(pids.into_iter().next())
}

use std::collections::HashMap;
use std::path::Path;

/// Check if a command exits with code 0 (readiness check).
pub fn check_command_readiness(
    command: &str,
    cwd: Option<&Path>,
    env: Option<&HashMap<String, String>>,
) -> bool {
    #[cfg(unix)]
    let mut cmd = Command::new("sh");
    #[cfg(unix)]
    cmd.arg("-c").arg(command);

    #[cfg(windows)]
    let mut cmd = Command::new("cmd");
    #[cfg(windows)]
    cmd.arg("/C").arg(command);

    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    if let Some(envs) = env {
        cmd.envs(envs);
    }
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());

    match cmd.status() {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

/// Wait until a TCP port, HTTP endpoint, or custom command becomes healthy (reachable/passing).
pub fn wait_for_health_full(
    port: Option<u16>,
    http_url: Option<&str>,
    command: Option<&str>,
    cwd: Option<&Path>,
    env: Option<&HashMap<String, String>>,
    timeout: Duration,
    interval: Duration,
) -> Result<(), String> {
    let start = Instant::now();

    while start.elapsed() < timeout {
        let mut healthy = true;

        if let Some(p) = port {
            if TcpStream::connect(("127.0.0.1", p)).is_err() {
                healthy = false;
            }
        }

        if healthy {
            if let Some(url) = http_url {
                if let Some(p) = extract_port_from_url(url) {
                    if TcpStream::connect(("127.0.0.1", p)).is_err() {
                        healthy = false;
                    }
                }
            }
        }

        if healthy {
            if let Some(cmd) = command {
                if !check_command_readiness(cmd, cwd, env) {
                    healthy = false;
                }
            }
        }

        if healthy && (port.is_some() || http_url.is_some() || command.is_some()) {
            return Ok(());
        }

        thread::sleep(interval);
    }

    Err(format!(
        "Health check timed out after {}s",
        timeout.as_secs()
    ))
}

/// Wait until a TCP port or HTTP endpoint becomes healthy (reachable).
pub fn wait_for_health(
    port: Option<u16>,
    http_url: Option<&str>,
    timeout: Duration,
    interval: Duration,
) -> Result<(), String> {
    wait_for_health_full(port, http_url, None, None, None, timeout, interval)
}

fn extract_port_from_url(url: &str) -> Option<u16> {
    let stripped = url.trim_start_matches("http://").trim_start_matches("https://");
    let host_port = stripped.split('/').next()?;
    let port_str = host_port.split(':').nth(1)?;
    port_str.parse::<u16>().ok()
}

