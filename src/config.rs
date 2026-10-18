use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServiceConfig {
    pub enabled: bool,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(alias = "working_dir")]
    pub cwd: PathBuf,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct DevboxConfig {
    #[serde(default)]
    pub version: u32,
    pub services: HashMap<String, ServiceConfig>,
}

impl DevboxConfig {
    pub fn enabled_services(&self) -> impl Iterator<Item = (&String, &ServiceConfig)> {
        self.services.iter().filter(|(_, service)| service.enabled)
    }
}

pub fn load_config(path: &Path) -> Result<DevboxConfig, Box<dyn error::Error>> {
    let contents = fs::read_to_string(path)?;
    let config: DevboxConfig = serde_yaml_ng::from_str(&contents)?;

    Ok(config)
}

pub fn find_config() -> Result<PathBuf, io::Error> {
    let devpane_path = std::env::current_dir()?.join("devpane.yaml");
    if devpane_path.is_file() {
        return Ok(devpane_path);
    }

    let devbox_path = std::env::current_dir()?.join("devbox.yaml");
    if devbox_path.is_file() {
        return Ok(devbox_path);
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "devpane.yaml (or devbox.yaml) not found in the current directory",
    ))
}
