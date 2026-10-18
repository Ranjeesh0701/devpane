use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProcessStatus {
    #[default]
    Running,
    Exited,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessSession {
    pub id: String,
    pub pid: u32,
    pub name: String,
    pub command: String,
    pub start_time_seconds: u64,
    pub start_time_useconds: u64,
    pub cwd: PathBuf,

    #[serde(default)]
    pub status: ProcessStatus,

    #[serde(default)]
    pub exit_code: Option<i32>,

    pub project_dir: Option<PathBuf>,
    #[serde(default)]
    pub cpu_percent: Option<f32>,
    #[serde(default)]
    pub memory_bytes: Option<u64>,
}
