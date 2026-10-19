use std::fmt;

#[derive(Debug)]
pub enum DevpaneError {
    InvalidArguments(String),
    ProcessLaunch(std::io::Error),
    ProcessNotFound(std::io::Error),
    FileOperation(std::io::Error),
    ProcessCheck(std::io::Error),
    DaemonError(String),
    Ipc(std::io::Error),
    InvalidResponse,
    Json(serde_json::Error),
    StorageDirectoryUnavailable,
    ConfigNotFound(std::io::Error),
    Tui(Box<dyn std::error::Error>),
    ExitCodeUnavailable,
    ExitCodeOutOfRange(i32),
}

impl fmt::Display for DevpaneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DevpaneError::InvalidArguments(message) => {
                write!(f, "{message}")
            }
            DevpaneError::ProcessLaunch(error) => {
                write!(f, "Failed to start process: {error}")
            }
            DevpaneError::ProcessCheck(error) => {
                write!(f, "Failed to check process: {error}")
            }
            DevpaneError::ExitCodeUnavailable => {
                write!(f, "Process terminated without a normal exit code")
            }
            DevpaneError::ExitCodeOutOfRange(code) => {
                write!(f, "Cannot represent child exit code: {code}")
            }
            DevpaneError::FileOperation(error) => {
                write!(f, "File operation failed: {error}")
            }
            DevpaneError::Ipc(error) => {
                write!(f, "IPC operation failed: {error}")
            }
            DevpaneError::Json(error) => {
                write!(f, "JSON operation failed: {error}")
            }
            DevpaneError::InvalidResponse => {
                write!(f, "Invalid response received")
            }
            DevpaneError::ProcessNotFound(error) => {
                write!(f, "Process not found: {error}")
            }
            DevpaneError::StorageDirectoryUnavailable => {
                write!(f, "Could not determine the application data directory")
            }
            DevpaneError::DaemonError(message) => {
                write!(f, "Daemon error: {message}")
            }
            DevpaneError::ConfigNotFound(error) => {
                write!(f, "Config not found: {error}")
            }
            DevpaneError::Tui(error) => {
                write!(f, "TUI error: {error}")
            }
        }
    }
}
