use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::Command;

/// Run a single shell command synchronously.
/// Output is appended to the specified log file (if provided) and errors are returned.
pub fn run_command(
    command_str: &str,
    cwd: &Path,
    env: &HashMap<String, String>,
    log_file_path: Option<&Path>,
) -> Result<(), String> {
    #[cfg(unix)]
    let mut cmd = Command::new("sh");
    #[cfg(unix)]
    cmd.arg("-c").arg(command_str);

    #[cfg(windows)]
    let mut cmd = Command::new("cmd");
    #[cfg(windows)]
    cmd.arg("/C").arg(command_str);

    cmd.current_dir(cwd);
    cmd.envs(env);

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute command '{command_str}': {e}"))?;

    if let Some(log_path) = log_file_path {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
            let _ = writeln!(file, "[devpane:hook] $ {}", command_str);
            if !output.stdout.is_empty() {
                let _ = file.write_all(&output.stdout);
            }
            if !output.stderr.is_empty() {
                let _ = file.write_all(&output.stderr);
            }
        }
    }

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let stderr_msg = String::from_utf8_lossy(&output.stderr);
        let error_detail = if stderr_msg.trim().is_empty() {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else {
            stderr_msg.trim().to_string()
        };

        return Err(format!(
            "Command '{}' exited with status {}: {}",
            command_str, code, error_detail
        ));
    }

    Ok(())
}

/// Run a list of shell commands in order. Stops at the first failure.
pub fn run_command_list(
    commands: &[String],
    cwd: &Path,
    env: &HashMap<String, String>,
    log_file_path: Option<&Path>,
) -> Result<(), String> {
    for cmd in commands {
        run_command(cmd, cwd, env, log_file_path)?;
    }
    Ok(())
}

/// Check if a build check command passes. Returns true if check command exits with 0.
pub fn check_build_required(
    check_command: &str,
    cwd: &Path,
    env: &HashMap<String, String>,
) -> bool {
    #[cfg(unix)]
    let mut cmd = Command::new("sh");
    #[cfg(unix)]
    cmd.arg("-c").arg(check_command);

    #[cfg(windows)]
    let mut cmd = Command::new("cmd");
    #[cfg(windows)]
    cmd.arg("/C").arg(check_command);

    cmd.current_dir(cwd);
    cmd.envs(env);
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());

    match cmd.status() {
        Ok(status) => !status.success(), // If check command exits 0, build is up-to-date (NOT required). If non-zero, build IS required.
        Err(_) => true,
    }
}
