use std::fs;
use std::path::PathBuf;

use super::error::DevpaneError;
use super::process::session::ProcessSession;

pub const DIR_NAME: &str = "devpane";
pub const SESSIONS: &str = "sessions.json";
pub const SESSIONS_TMP: &str = "sessions.json.tmp";

pub fn get_storage_directory() -> Result<PathBuf, DevpaneError> {
    let Some(mut path) = dirs::data_dir() else {
        return Err(DevpaneError::StorageDirectoryUnavailable);
    };
    path.push(DIR_NAME);

    Ok(path)
}

pub fn get_logs_directory() -> Result<PathBuf, DevpaneError> {
    let mut path = get_storage_directory()?;
    path.push(format!("logs"));

    Ok(path)
}

pub fn ensure_storage_directory() -> Result<PathBuf, DevpaneError> {
    let path = get_storage_directory()?;

    if !path.exists() {
        fs::create_dir_all(&path).map_err(DevpaneError::FileOperation)?;
    }

    Ok(path)
}

pub fn ensure_logs_directory() -> Result<PathBuf, DevpaneError> {
    let path = get_logs_directory()?;

    if !path.exists() {
        fs::create_dir_all(&path).map_err(DevpaneError::FileOperation)?;
    }

    Ok(path)
}

pub fn get_log_file(id: &str) -> Result<PathBuf, DevpaneError> {
    let mut path = ensure_logs_directory()?;
    path.push(format!("{id}.log"));

    Ok(path)
}

pub fn get_session(id: &str) -> Result<ProcessSession, DevpaneError> {
    let sessions = get_sessions()?;
    sessions.into_iter().find(|s| s.id == id).ok_or_else(|| {
        DevpaneError::ProcessNotFound(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Session with ID {} not found", id),
        ))
    })
}

pub fn get_sessions() -> Result<Vec<ProcessSession>, DevpaneError> {
    let storage_directory = get_storage_directory()?;
    let sessions_file = storage_directory.join(SESSIONS);

    let text = match fs::read_to_string(sessions_file) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Vec::new());
        }
        Err(error) => return Err(DevpaneError::FileOperation(error)),
    };

    let sessions: Vec<ProcessSession> = serde_json::from_str(&text).map_err(DevpaneError::Json)?;

    Ok(sessions)
}

pub fn save_sessions(sessions: &[ProcessSession]) -> Result<(), DevpaneError> {
    let json = serde_json::to_string(sessions).map_err(DevpaneError::Json)?;
    let storage_directory = get_storage_directory()?;
    let temp_file = storage_directory.join(SESSIONS_TMP);
    let file = storage_directory.join(SESSIONS);
    fs::write(&temp_file, json).map_err(DevpaneError::FileOperation)?;
    fs::rename(&temp_file, &file).map_err(DevpaneError::FileOperation)?;

    Ok(())
}

pub fn remove_session(id: String) -> Result<(), DevpaneError> {
    let mut sessions = get_sessions()?;
    sessions.retain(|s| s.id != id);

    save_sessions(&sessions)?;

    Ok(())
}
