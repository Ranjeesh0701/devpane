use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum CommandList {
    Single(String),
    Multiple(Vec<String>),
}

impl CommandList {
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            Self::Single(s) => vec![s.clone()],
            Self::Multiple(list) => list.clone(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct LifecycleHooks {
    #[serde(default)]
    pub pre_build: Option<CommandList>,
    #[serde(default)]
    pub post_build: Option<CommandList>,
    #[serde(default)]
    pub pre_start: Option<CommandList>,
    #[serde(default)]
    pub post_start: Option<CommandList>,
    #[serde(default)]
    pub pre_stop: Option<CommandList>,
    #[serde(default)]
    pub post_stop: Option<CommandList>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct BuildConfig {
    #[serde(default)]
    pub check: Option<CommandList>,
    #[serde(default)]
    pub cmd: Option<CommandList>,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RestartMode {
    #[default]
    #[serde(alias = "never", alias = "no")]
    Never,
    #[serde(alias = "always")]
    Always,
    #[serde(alias = "on-failure", alias = "crash")]
    Crash,
    #[serde(alias = "on-change", alias = "change")]
    Change,
    #[serde(alias = "manual")]
    Manual,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct RestartConfig {
    #[serde(default)]
    pub on: RestartMode,
    #[serde(default = "default_max_crash_retries")]
    pub max_crash_retries: u32,
    #[serde(default = "default_backoff_ms")]
    pub backoff_ms: u64,
    #[serde(default = "default_restart_debounce_ms")]
    pub debounce_ms: u64,
}

fn default_max_crash_retries() -> u32 {
    5
}

fn default_backoff_ms() -> u64 {
    1000
}

fn default_restart_debounce_ms() -> u64 {
    300
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum RestartOpt {
    Mode(RestartMode),
    Detailed(RestartConfig),
}

impl RestartOpt {
    pub fn into_config(self) -> RestartConfig {
        match self {
            Self::Mode(mode) => RestartConfig {
                on: mode,
                max_crash_retries: default_max_crash_retries(),
                backoff_ms: default_backoff_ms(),
                debounce_ms: default_restart_debounce_ms(),
            },
            Self::Detailed(config) => config,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct WatchConfig {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub ignore_paths: Vec<String>,
    #[serde(default = "default_watch_debounce_ms")]
    pub debounce_ms: u64,
}

fn default_watch_debounce_ms() -> u64 {
    300
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum WatchOpt {
    Enabled(bool),
    Paths(Vec<String>),
    Detailed(WatchConfig),
}

impl WatchOpt {
    pub fn into_config(self) -> Option<WatchConfig> {
        match self {
            Self::Enabled(true) => Some(WatchConfig {
                include: vec![".".to_string()],
                ..Default::default()
            }),
            Self::Enabled(false) => None,
            Self::Paths(paths) => Some(WatchConfig {
                include: paths,
                ..Default::default()
            }),
            Self::Detailed(cfg) => Some(cfg),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyCondition {
    #[default]
    #[serde(alias = "ready")]
    Ready,
    #[serde(alias = "started")]
    Started,
    #[serde(alias = "running")]
    Running,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct DependencySpec {
    #[serde(default)]
    pub condition: DependencyCondition,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum DependsOnConfig {
    List(Vec<String>),
    Map(HashMap<String, DependencySpec>),
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct HealthCheckConfig {
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub http: Option<String>,
    #[serde(default)]
    pub command: Option<CommandList>,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_interval_ms")]
    pub interval_ms: u64,
    #[serde(default = "default_retries")]
    pub retries: u32,
}

fn default_timeout_secs() -> u64 {
    30
}

fn default_interval_ms() -> u64 {
    250
}

fn default_retries() -> u32 {
    3
}

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
    #[serde(default)]
    pub depends_on: Option<DependsOnConfig>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub health_check: Option<HealthCheckConfig>,
    #[serde(default)]
    pub restart: Option<RestartOpt>,
    #[serde(default)]
    pub watch: Option<WatchOpt>,
    #[serde(default)]
    pub hooks: Option<LifecycleHooks>,
    #[serde(default)]
    pub build: Option<BuildConfig>,
}

impl ServiceConfig {
    pub fn dependency_map(&self) -> HashMap<String, DependencySpec> {
        match &self.depends_on {
            None => HashMap::new(),
            Some(DependsOnConfig::List(list)) => list
                .iter()
                .map(|name| {
                    (
                        name.clone(),
                        DependencySpec {
                            condition: DependencyCondition::Ready,
                            timeout_secs: None,
                        },
                    )
                })
                .collect(),
            Some(DependsOnConfig::Map(map)) => map.clone(),
        }
    }

    pub fn dependency_names(&self) -> Vec<String> {
        match &self.depends_on {
            None => Vec::new(),
            Some(DependsOnConfig::List(list)) => list.clone(),
            Some(DependsOnConfig::Map(map)) => {
                let mut keys: Vec<String> = map.keys().cloned().collect();
                keys.sort();
                keys
            }
        }
    }

    pub fn restart_config(&self) -> Option<RestartConfig> {
        self.restart.clone().map(|r| r.into_config())
    }

    pub fn watch_config(&self) -> Option<WatchConfig> {
        self.watch.clone().and_then(|w| w.into_config())
    }
}

#[derive(Debug, Deserialize)]
pub struct DevpaneConfig {
    #[serde(default)]
    pub version: u32,
    pub services: HashMap<String, ServiceConfig>,
}

impl DevpaneConfig {
    pub fn enabled_services(&self) -> impl Iterator<Item = (&String, &ServiceConfig)> {
        self.services.iter().filter(|(_, service)| service.enabled)
    }
}

pub fn load_config(path: &Path) -> Result<DevpaneConfig, Box<dyn error::Error>> {
    let contents = fs::read_to_string(path)?;
    let config: DevpaneConfig = serde_yaml_ng::from_str(&contents)?;

    Ok(config)
}

/// Automatically loads variables from .env and .env.local in the specified directory.
pub fn load_env_files(dir: &Path) -> HashMap<String, String> {
    let mut env = HashMap::new();

    // 1. Load .env
    let env_path = dir.join(".env");
    if let Ok(contents) = fs::read_to_string(&env_path) {
        parse_env_file_into(&contents, &mut env);
    }

    // 2. Load .env.local (takes precedence over .env)
    let env_local_path = dir.join(".env.local");
    if let Ok(contents) = fs::read_to_string(&env_local_path) {
        parse_env_file_into(&contents, &mut env);
    }

    env
}

fn parse_env_file_into(contents: &str, map: &mut HashMap<String, String>) {
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some((key, val)) = trimmed.split_once('=') {
            let key = key.trim();
            if key.is_empty() {
                continue;
            }
            let mut val = val.trim();
            if (val.starts_with('"') && val.ends_with('"'))
                || (val.starts_with('\'') && val.ends_with('\''))
            {
                if val.len() >= 2 {
                    val = &val[1..val.len() - 1];
                }
            }
            map.insert(key.to_string(), val.to_string());
        }
    }
}

pub fn find_config() -> Result<PathBuf, io::Error> {
    let current = std::env::current_dir()?;
    let candidates = [
        "devpane.yaml",
        "devpane.yml",
    ];

    for candidate in candidates {
        let path = current.join(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "devpane.yaml (or devpane.yml) not found in the current directory",
    ))
}

pub fn init_config(
    dir: &Path,
    force: bool,
) -> Result<(PathBuf, Vec<(String, String)>), Box<dyn error::Error>> {
    let target_path = dir.join("devpane.yaml");
    if target_path.exists() && !force {
        return Err(format!(
            "devpane.yaml already exists at {}.\nUse --force to overwrite.",
            target_path.display()
        )
        .into());
    }

    let mut detected_services: Vec<(String, String, Vec<String>)> = Vec::new();

    // 1. Check for Procfile
    let procfile = dir.join("Procfile");
    if procfile.is_file() {
        if let Ok(content) = fs::read_to_string(&procfile) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if let Some((name, cmd_str)) = trimmed.split_once(':') {
                    let service_name = name.trim().to_string();
                    let parts: Vec<String> = cmd_str
                        .split_whitespace()
                        .map(|s| s.to_string())
                        .collect();
                    if !parts.is_empty() {
                        let program = parts[0].clone();
                        let args = parts[1..].to_vec();
                        detected_services.push((service_name, program, args));
                    }
                }
            }
        }
    }

    // 2. Check for package.json
    let package_json = dir.join("package.json");
    if package_json.is_file() && detected_services.is_empty() {
        if let Ok(content) = fs::read_to_string(&package_json) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                let pkg_mgr = if dir.join("bun.lock").exists() || dir.join("bun.lockb").exists() {
                    "bun"
                } else if dir.join("pnpm-lock.yaml").exists() {
                    "pnpm"
                } else if dir.join("yarn.lock").exists() {
                    "yarn"
                } else {
                    "npm"
                };

                if let Some(scripts) = val.get("scripts").and_then(|s| s.as_object()) {
                    if scripts.contains_key("dev") {
                        detected_services.push((
                            "web".to_string(),
                            pkg_mgr.to_string(),
                            vec!["run".to_string(), "dev".to_string()],
                        ));
                    } else if scripts.contains_key("start") {
                        detected_services.push((
                            "web".to_string(),
                            pkg_mgr.to_string(),
                            vec!["start".to_string()],
                        ));
                    }
                }
            }
        }
    }

    // 3. Check for Cargo.toml
    let cargo_toml = dir.join("Cargo.toml");
    if cargo_toml.is_file() && detected_services.is_empty() {
        detected_services.push((
            "app".to_string(),
            "cargo".to_string(),
            vec!["run".to_string()],
        ));
    }

    // 4. Check for go.mod
    let go_mod = dir.join("go.mod");
    if go_mod.is_file() && detected_services.is_empty() {
        detected_services.push((
            "api".to_string(),
            "go".to_string(),
            vec!["run".to_string(), ".".to_string()],
        ));
    }

    // 5. Check for Python projects
    if detected_services.is_empty() {
        if dir.join("manage.py").is_file() {
            detected_services.push((
                "django".to_string(),
                "python".to_string(),
                vec!["manage.py".to_string(), "runserver".to_string()],
            ));
        } else if dir.join("requirements.txt").is_file()
            || dir.join("pyproject.toml").is_file()
            || dir.join("Pipfile").is_file()
        {
            if dir.join("main.py").is_file() {
                detected_services.push((
                    "app".to_string(),
                    "python".to_string(),
                    vec!["main.py".to_string()],
                ));
            } else if dir.join("app.py").is_file() {
                detected_services.push((
                    "app".to_string(),
                    "python".to_string(),
                    vec!["app.py".to_string()],
                ));
            }
        }
    }

    // 6. Check for docker-compose.yml
    if detected_services.is_empty()
        && (dir.join("docker-compose.yml").is_file() || dir.join("compose.yaml").is_file())
    {
        detected_services.push((
            "docker".to_string(),
            "docker".to_string(),
            vec!["compose".to_string(), "up".to_string()],
        ));
    }

    // 7. Fallback template if nothing detected
    let is_starter = detected_services.is_empty();
    if is_starter {
        detected_services.push((
            "web".to_string(),
            "sh".to_string(),
            vec![
                "-c".to_string(),
                "while true; do echo '[web] Server listening on http://localhost:3000'; sleep 2; done".to_string(),
            ],
        ));
        detected_services.push((
            "worker".to_string(),
            "sh".to_string(),
            vec![
                "-c".to_string(),
                "while true; do echo '[worker] Background worker running...'; sleep 3; done".to_string(),
            ],
        ));
    }

    // Build the YAML content
    let mut yaml = String::new();
    yaml.push_str("# Devpane Configuration\n");
    yaml.push_str("# Run 'devpane up' to start all enabled services with interactive terminal tabs.\n\n");
    yaml.push_str("services:\n");

    let mut summary = Vec::new();
    for (name, prog, args) in &detected_services {
        let cmd_summary = if args.is_empty() {
            prog.clone()
        } else {
            format!("{} {}", prog, args.join(" "))
        };
        summary.push((name.clone(), cmd_summary));

        yaml.push_str(&format!("  {}:\n", name));
        yaml.push_str(&format!("    command: \"{}\"\n", prog));
        if args.is_empty() {
            yaml.push_str("    args: []\n");
        } else {
            yaml.push_str("    args:\n");
            for arg in args {
                yaml.push_str(&format!("      - \"{}\"\n", arg.replace('\"', "\\\"")));
            }
        }
        yaml.push_str("    working_dir: .\n");
        yaml.push_str("    enabled: true\n\n");
    }

    fs::write(&target_path, yaml)?;

    Ok((target_path, summary))
}
