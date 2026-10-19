use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::PathBuf;

use super::config::ServiceConfig;
use super::process::session::ProcessSession;

const MAX_MESSAGE_LEN: usize = 1024 * 1024; // 1 MB

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    Start {
        name: String,
        program: String,
        args: Vec<String>,
        cwd: PathBuf,
        env: HashMap<String, String>,
    },
    List,
    Stop {
        id: String,
    },
    Logs {
        id: String,
        follow: bool,
    },
    Shutdown,
    Up {
        project_dir: PathBuf,
        services: HashMap<String, ServiceConfig>,
    },
    Down {
        project_dir: PathBuf,
    },
    Restart {
        name: Option<String>,
        project_dir: Option<PathBuf>,
        services: Option<HashMap<String, ServiceConfig>>,
    },
    Rebuild {
        name: String,
    },
    Sync {
        project_dir: PathBuf,
        install_deps: bool,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Start { session: ProcessSession },
    List { sessions: Vec<ProcessSession> },
    Stop,
    Logs { logs: String },
    LogChunk { logs: String },
    LogStreamEnd,
    Shutdown,
    Error { message: String },
    Up { results: Vec<ServiceStartResult> },
    Down { results: Vec<ServiceStopResult> },
    Restart { results: Vec<ServiceStartResult> },
    Rebuild { success: bool, error: Option<String> },
    Sync { report: crate::sync::SyncReport },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceStartResult {
    pub name: String,
    pub session: Option<ProcessSession>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceStopResult {
    pub name: String,
    pub error: Option<String>,
}

pub fn write_message<W: Write, T: Serialize>(writer: &mut W, message: &T) -> io::Result<()> {
    let payload = serde_json::to_vec(message)?;

    if payload.len() > MAX_MESSAGE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Message too large",
        ));
    }

    let len = u32::try_from(payload.len()).expect("Payload is too large");

    writer.write_all(&len.to_be_bytes())?;
    writer.write_all(&payload)
}

pub fn read_message<R: Read, T: DeserializeOwned>(reader: &mut R) -> io::Result<T> {
    let mut header = [0u8; 4];
    reader.read_exact(&mut header)?;

    let len = u32::from_be_bytes(header) as usize;
    if len > MAX_MESSAGE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Message too large",
        ));
    }

    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload)?;

    Ok(serde_json::from_slice(&payload)?)
}
