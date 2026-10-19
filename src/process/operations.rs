use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::{fs, io};

use super::platform;
use super::session::{ProcessSession, ProcessStatus};
use crate::error::DevboxError;
use crate::storage::get_log_file;
use uuid::Uuid;

pub fn start_process(
    name: String,
    program: String,
    args: Vec<String>,
    cwd: PathBuf,
    env: HashMap<String, String>,
    project_dir: Option<PathBuf>,
) -> Result<(Child, ProcessSession), DevboxError> {
    let id = Uuid::new_v4().to_string();

    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(get_log_file(&id)?)
        .map_err(DevboxError::FileOperation)?;

    let stdout_file = file.try_clone().map_err(DevboxError::FileOperation)?;

    let mut cmd = Command::new(&program);
    cmd.args(args)
        .current_dir(&cwd)
        .envs(env)
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(file));

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    let child = cmd.spawn().map_err(DevboxError::ProcessLaunch)?;

    let pid = child.id();

    let (start_time_seconds, start_time_useconds) = match platform::get_process_start_time(pid) {
        Ok(value) => value,
        Err(error) => {
            let _ = platform::stop_process(pid);
            return Err(DevboxError::ProcessCheck(error));
        }
    };

    let session = ProcessSession {
        id,
        pid,
        name,
        command: program,
        start_time_seconds,
        start_time_useconds,
        status: ProcessStatus::Running,
        exit_code: None,
        project_dir,
        cwd,
        cpu_percent: None,
        memory_bytes: None,
    };

    println!("Started {} with PID {}", session.name, session.pid);

    Ok((child, session))
}

pub use platform::stop_process;

pub fn is_same_process(session: &ProcessSession) -> Result<bool, io::Error> {
    if !platform::is_process_active(session.pid)? {
        return Ok(false);
    }

    let (current_start_time_seconds, current_start_time_useconds) =
        platform::get_process_start_time(session.pid)?;

    Ok(current_start_time_seconds == session.start_time_seconds
        && current_start_time_useconds == session.start_time_useconds)
}
