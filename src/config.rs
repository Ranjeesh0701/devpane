use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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

#[derive(Debug, Clone)]
pub struct DetectedService {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: String,
    pub port: Option<u16>,
    pub watch_paths: Vec<String>,
    pub build_cmd: Option<String>,
    pub depends_on: Vec<String>,
}

pub fn init_config(
    dir: &Path,
    force: bool,
) -> Result<(PathBuf, Vec<(String, String)>), Box<dyn error::Error>> {
    init_config_with_opts(dir, force, false)
}

pub fn init_config_with_opts(
    dir: &Path,
    force: bool,
    import_compose: bool,
) -> Result<(PathBuf, Vec<(String, String)>), Box<dyn error::Error>> {
    let target_path = dir.join("devpane.yaml");
    if target_path.exists() && !force {
        return Err(format!(
            "devpane.yaml already exists at {}.\nUse --force to overwrite.",
            target_path.display()
        )
        .into());
    }

    let mut detected_services: Vec<DetectedService> = Vec::new();

    // 1. If import_compose is explicitly requested
    if import_compose {
        if let Some(services) = parse_docker_compose(dir) {
            detected_services = services;
        } else {
            return Err("No docker-compose.yml or compose.yaml found in the current directory.".into());
        }
    }

    // 2. Check for Procfile
    if detected_services.is_empty() {
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
                            detected_services.push(DetectedService {
                                name: service_name,
                                command: program,
                                args,
                                working_dir: ".".to_string(),
                                port: None,
                                watch_paths: Vec::new(),
                                build_cmd: None,
                                depends_on: Vec::new(),
                            });
                        }
                    }
                }
            }
        }
    }

    // 3. Scan root directory services
    if detected_services.is_empty() {
        detect_services_in_dir(dir, ".", &mut detected_services);
    }

    // 4. Scan common subdirectories (monorepo & fullstack patterns)
    let direct_subdirs = [
        "frontend", "backend", "client", "server", "web", "api", "ui",
    ];
    for sub in &direct_subdirs {
        let sub_path = dir.join(sub);
        if sub_path.is_dir() {
            detect_services_in_dir(&sub_path, &format!("./{}", sub), &mut detected_services);
        }
    }

    let container_dirs = ["apps", "packages", "services"];
    for parent in &container_dirs {
        let parent_path = dir.join(parent);
        if parent_path.is_dir() {
            if let Ok(entries) = fs::read_dir(&parent_path) {
                for entry in entries.flatten() {
                    let child_path = entry.path();
                    if child_path.is_dir() {
                        if let Some(child_name) = child_path.file_name().and_then(|n| n.to_str()) {
                            let rel = format!("./{}/{}", parent, child_name);
                            detect_services_in_dir(&child_path, &rel, &mut detected_services);
                        }
                    }
                }
            }
        }
    }

    // 5. If still empty, check for Docker Compose as fallback
    if detected_services.is_empty() {
        if let Some(services) = parse_docker_compose(dir) {
            detected_services = services;
        }
    }

    // 6. Fallback starter template if nothing detected
    if detected_services.is_empty() {
        detected_services.push(DetectedService {
            name: "web".to_string(),
            command: "sh".to_string(),
            args: vec![
                "-c".to_string(),
                "while true; do echo '[web] Server listening on http://localhost:3000'; sleep 2; done".to_string(),
            ],
            working_dir: ".".to_string(),
            port: Some(3000),
            watch_paths: vec!["src".to_string()],
            build_cmd: None,
            depends_on: Vec::new(),
        });
        detected_services.push(DetectedService {
            name: "worker".to_string(),
            command: "sh".to_string(),
            args: vec![
                "-c".to_string(),
                "while true; do echo '[worker] Background worker running...'; sleep 3; done".to_string(),
            ],
            working_dir: ".".to_string(),
            port: None,
            watch_paths: vec!["src".to_string()],
            build_cmd: None,
            depends_on: Vec::new(),
        });
    }

    // Ensure all service names are unique
    let mut seen_names = HashSet::new();
    for svc in &mut detected_services {
        if seen_names.contains(&svc.name) {
            let mut counter = 2;
            let base = svc.name.clone();
            while seen_names.contains(&format!("{}-{}", base, counter)) {
                counter += 1;
            }
            svc.name = format!("{}-{}", base, counter);
        }
        seen_names.insert(svc.name.clone());
    }

    // Automatic dependency linking (e.g. frontend -> backend)
    let has_backend = detected_services.iter().any(|s| s.name == "api" || s.name == "backend" || s.name == "server");
    let has_db = detected_services.iter().any(|s| s.name == "db" || s.name == "database" || s.name == "postgres" || s.name == "redis");
    for svc in &mut detected_services {
        if (svc.name == "web" || svc.name == "frontend" || svc.name == "client") && svc.depends_on.is_empty() {
            if has_backend {
                svc.depends_on.push("api".to_string());
            } else if has_db {
                svc.depends_on.push("db".to_string());
            }
        }
    }

    let has_env = dir.join(".env").is_file();
    let has_env_example = dir.join(".env.example").is_file();

    let mut yaml = String::new();
    yaml.push_str("# Devpane Configuration\n");
    yaml.push_str("# Run 'devpane up' to start all enabled services with interactive terminal tabs.\n");
    yaml.push_str("# TUI Controls: 'r' restart | 'b' rebuild | 's' stop/start | Tab switch tabs\n\n");
    yaml.push_str("services:\n");

    let mut summary = Vec::new();
    for svc in &detected_services {
        let cmd_summary = if svc.args.is_empty() {
            svc.command.clone()
        } else {
            format!("{} {}", svc.command, svc.args.join(" "))
        };
        summary.push((svc.name.clone(), cmd_summary));

        yaml.push_str(&format!("  {}:\n", svc.name));
        yaml.push_str(&format!("    command: \"{}\"\n", svc.command));
        if svc.args.is_empty() {
            yaml.push_str("    args: []\n");
        } else {
            yaml.push_str("    args:\n");
            for arg in &svc.args {
                yaml.push_str(&format!("      - \"{}\"\n", arg.replace('\"', "\\\"")));
            }
        }
        yaml.push_str(&format!("    working_dir: {}\n", svc.working_dir));
        yaml.push_str("    enabled: true\n");
        yaml.push_str("    restart: on-failure\n");

        if !svc.watch_paths.is_empty() {
            yaml.push_str("    watch:\n");
            yaml.push_str("      include:\n");
            for w in &svc.watch_paths {
                yaml.push_str(&format!("        - \"{}\"\n", w));
            }
            yaml.push_str("      debounce_ms: 300\n");
        }

        if let Some(port) = svc.port {
            yaml.push_str("    health_check:\n");
            yaml.push_str(&format!("      port: {}\n", port));
            yaml.push_str("      timeout_secs: 20\n");
        }

        if !svc.depends_on.is_empty() {
            yaml.push_str("    depends_on:\n");
            for dep in &svc.depends_on {
                yaml.push_str(&format!("      - {}\n", dep));
            }
        }

        if let Some(ref build) = svc.build_cmd {
            yaml.push_str("    # build:\n");
            yaml.push_str(&format!("    #   cmd: \"{}\"\n", build));
        }

        if has_env {
            yaml.push_str("    # env_file: .env\n");
        } else if has_env_example {
            yaml.push_str("    # env_file: .env # (copy from .env.example)\n");
        }

        yaml.push('\n');
    }

    fs::write(&target_path, yaml)?;

    Ok((target_path, summary))
}

fn parse_docker_compose(dir: &Path) -> Option<Vec<DetectedService>> {
    let compose_files = [
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ];

    let mut compose_path = None;
    for filename in &compose_files {
        let p = dir.join(filename);
        if p.is_file() {
            compose_path = Some(p);
            break;
        }
    }

    let path = compose_path?;
    let content = fs::read_to_string(&path).ok()?;
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&content).ok()?;
    let services = yaml.get("services")?.as_mapping()?;
    let mut detected = Vec::new();

    for (k, v) in services {
        let name = k.as_str()?.to_string();
        let mut command = String::new();
        let mut args = Vec::new();
        let mut working_dir = ".".to_string();
        let mut port = None;
        let mut depends_on = Vec::new();

        // Check if build context is specified
        if let Some(build_val) = v.get("build") {
            if let Some(ctx) = build_val.as_str() {
                working_dir = ctx.to_string();
            } else if let Some(ctx) = build_val.get("context").and_then(|c| c.as_str()) {
                working_dir = ctx.to_string();
            }
        }

        // Check command
        if let Some(cmd_val) = v.get("command") {
            if let Some(cmd_str) = cmd_val.as_str() {
                let parts: Vec<String> = cmd_str
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();
                if !parts.is_empty() {
                    command = parts[0].clone();
                    args = parts[1..].to_vec();
                }
            } else if let Some(cmd_seq) = cmd_val.as_sequence() {
                let parts: Vec<String> = cmd_seq
                    .iter()
                    .filter_map(|s| s.as_str().map(|str| str.to_string()))
                    .collect();
                if !parts.is_empty() {
                    command = parts[0].clone();
                    args = parts[1..].to_vec();
                }
            }
        }

        // If no explicit command but image is specified
        if command.is_empty() {
            if let Some(_image) = v.get("image").and_then(|i| i.as_str()) {
                command = "docker".to_string();
                args = vec!["compose".to_string(), "up".to_string(), name.clone()];
            } else {
                command = "sh".to_string();
                args = vec!["-c".to_string(), format!("echo 'Starting {}'", name)];
            }
        }

        // Check ports
        if let Some(ports_seq) = v.get("ports").and_then(|p| p.as_sequence()) {
            for p in ports_seq {
                if let Some(p_str) = p.as_str() {
                    let host_port_part = p_str.split(':').next().unwrap_or(p_str);
                    if let Ok(parsed) = host_port_part.trim().parse::<u16>() {
                        port = Some(parsed);
                        break;
                    }
                } else if let Some(p_num) = p.as_u64() {
                    port = Some(p_num as u16);
                    break;
                }
            }
        }

        // Check depends_on
        if let Some(deps_val) = v.get("depends_on") {
            if let Some(deps_seq) = deps_val.as_sequence() {
                for d in deps_seq {
                    if let Some(dep_name) = d.as_str() {
                        depends_on.push(dep_name.to_string());
                    }
                }
            } else if let Some(deps_map) = deps_val.as_mapping() {
                for (dk, _) in deps_map {
                    if let Some(dep_name) = dk.as_str() {
                        depends_on.push(dep_name.to_string());
                    }
                }
            }
        }

        detected.push(DetectedService {
            name,
            command,
            args,
            working_dir: if working_dir.is_empty() { ".".to_string() } else { working_dir },
            port,
            watch_paths: Vec::new(),
            build_cmd: None,
            depends_on,
        });
    }

    if detected.is_empty() {
        None
    } else {
        Some(detected)
    }
}

fn detect_services_in_dir(
    dir: &Path,
    rel_path: &str,
    detected: &mut Vec<DetectedService>,
) {
    let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let is_root = rel_path == ".";

    // 1. Check for package.json
    let package_json = dir.join("package.json");
    if package_json.is_file() {
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

                let mut port = Some(3000);
                let mut watch = vec!["src".to_string()];
                let mut build_cmd = None;

                let all_deps = {
                    let mut s = String::new();
                    if let Some(deps) = val.get("dependencies").and_then(|d| d.as_object()) {
                        for k in deps.keys() { s.push_str(k); s.push(' '); }
                    }
                    if let Some(dev_deps) = val.get("devDependencies").and_then(|d| d.as_object()) {
                        for k in dev_deps.keys() { s.push_str(k); s.push(' '); }
                    }
                    s
                };

                if all_deps.contains("vite") {
                    port = Some(5173);
                    watch = vec!["src".to_string(), "public".to_string()];
                    build_cmd = Some(format!("{} run build", pkg_mgr));
                } else if all_deps.contains("next") {
                    port = Some(3000);
                    watch = vec!["src".to_string(), "pages".to_string(), "app".to_string(), "public".to_string()];
                    build_cmd = Some(format!("{} run build", pkg_mgr));
                } else if all_deps.contains("nuxt") {
                    port = Some(3000);
                    watch = vec!["pages".to_string(), "components".to_string(), "app.vue".to_string()];
                    build_cmd = Some(format!("{} run build", pkg_mgr));
                } else if all_deps.contains("astro") {
                    port = Some(4321);
                    watch = vec!["src".to_string(), "public".to_string()];
                    build_cmd = Some(format!("{} run build", pkg_mgr));
                } else if all_deps.contains("remix") {
                    port = Some(3000);
                    watch = vec!["app".to_string()];
                    build_cmd = Some(format!("{} run build", pkg_mgr));
                } else if all_deps.contains("svelte") {
                    port = Some(5173);
                    watch = vec!["src".to_string()];
                    build_cmd = Some(format!("{} run build", pkg_mgr));
                }

                if let Some(scripts) = val.get("scripts").and_then(|s| s.as_object()) {
                    let primary_name = if is_root {
                        "web".to_string()
                    } else if dir_name == "frontend" || dir_name == "client" || dir_name == "ui" || dir_name == "web" {
                        "web".to_string()
                    } else {
                        dir_name.to_string()
                    };

                    if scripts.contains_key("dev") {
                        detected.push(DetectedService {
                            name: primary_name,
                            command: pkg_mgr.to_string(),
                            args: vec!["run".to_string(), "dev".to_string()],
                            working_dir: rel_path.to_string(),
                            port,
                            watch_paths: watch,
                            build_cmd,
                            depends_on: Vec::new(),
                        });
                    } else if scripts.contains_key("start") {
                        detected.push(DetectedService {
                            name: primary_name,
                            command: pkg_mgr.to_string(),
                            args: vec!["start".to_string()],
                            working_dir: rel_path.to_string(),
                            port,
                            watch_paths: watch,
                            build_cmd,
                            depends_on: Vec::new(),
                        });
                    }

                    if scripts.contains_key("server") || scripts.contains_key("api") {
                        let script_key = if scripts.contains_key("api") { "api" } else { "server" };
                        let api_name = if is_root { "api".to_string() } else { format!("{}-api", dir_name) };
                        detected.push(DetectedService {
                            name: api_name,
                            command: pkg_mgr.to_string(),
                            args: vec!["run".to_string(), script_key.to_string()],
                            working_dir: rel_path.to_string(),
                            port: Some(4000),
                            watch_paths: vec!["src".to_string()],
                            build_cmd: None,
                            depends_on: Vec::new(),
                        });
                    }

                    if scripts.contains_key("worker") {
                        let worker_name = if is_root { "worker".to_string() } else { format!("{}-worker", dir_name) };
                        detected.push(DetectedService {
                            name: worker_name,
                            command: pkg_mgr.to_string(),
                            args: vec!["run".to_string(), "worker".to_string()],
                            working_dir: rel_path.to_string(),
                            port: None,
                            watch_paths: vec!["src".to_string()],
                            build_cmd: None,
                            depends_on: Vec::new(),
                        });
                    }
                }
            }
        }
    }

    // 2. Check for Cargo.toml
    let cargo_toml = dir.join("Cargo.toml");
    if cargo_toml.is_file() {
        let name = if is_root {
            "app".to_string()
        } else if dir_name == "backend" || dir_name == "server" || dir_name == "api" {
            "api".to_string()
        } else {
            dir_name.to_string()
        };
        detected.push(DetectedService {
            name,
            command: "cargo".to_string(),
            args: vec!["run".to_string()],
            working_dir: rel_path.to_string(),
            port: Some(8080),
            watch_paths: vec!["src".to_string()],
            build_cmd: Some("cargo build".to_string()),
            depends_on: Vec::new(),
        });
    }

    // 3. Check for go.mod
    let go_mod = dir.join("go.mod");
    if go_mod.is_file() {
        let name = if is_root {
            "api".to_string()
        } else if dir_name == "backend" || dir_name == "server" {
            "api".to_string()
        } else {
            dir_name.to_string()
        };
        let (cmd, args) = if dir.join(".air.toml").is_file() {
            ("air".to_string(), vec![])
        } else {
            ("go".to_string(), vec!["run".to_string(), ".".to_string()])
        };
        detected.push(DetectedService {
            name,
            command: cmd,
            args,
            working_dir: rel_path.to_string(),
            port: Some(8080),
            watch_paths: vec![".".to_string()],
            build_cmd: Some("go build .".to_string()),
            depends_on: Vec::new(),
        });
    }

    // 4. Check for Python projects
    let manage_py = dir.join("manage.py");
    let reqs = dir.join("requirements.txt");
    let pyproject = dir.join("pyproject.toml");
    let pipfile = dir.join("Pipfile");
    let main_py = dir.join("main.py");
    let app_py = dir.join("app.py");

    if manage_py.is_file() {
        detected.push(DetectedService {
            name: if is_root { "django".to_string() } else { format!("{}-django", dir_name) },
            command: "python".to_string(),
            args: vec!["manage.py".to_string(), "runserver".to_string()],
            working_dir: rel_path.to_string(),
            port: Some(8000),
            watch_paths: vec![".".to_string()],
            build_cmd: None,
            depends_on: Vec::new(),
        });
    } else if reqs.is_file() || pyproject.is_file() || pipfile.is_file() || main_py.is_file() || app_py.is_file() {
        let content = if reqs.is_file() {
            fs::read_to_string(&reqs).unwrap_or_default()
        } else if pyproject.is_file() {
            fs::read_to_string(&pyproject).unwrap_or_default()
        } else if pipfile.is_file() {
            fs::read_to_string(&pipfile).unwrap_or_default()
        } else {
            String::new()
        };

        let content_lower = content.to_lowercase();
        if content_lower.contains("fastapi") {
            let entry = if dir.join("app").join("main.py").is_file() {
                "app.main:app"
            } else {
                "main:app"
            };
            detected.push(DetectedService {
                name: if is_root { "api".to_string() } else { dir_name.to_string() },
                command: "uvicorn".to_string(),
                args: vec![entry.to_string(), "--reload".to_string()],
                working_dir: rel_path.to_string(),
                port: Some(8000),
                watch_paths: vec![".".to_string()],
                build_cmd: None,
                depends_on: Vec::new(),
            });
        } else if content_lower.contains("flask") {
            detected.push(DetectedService {
                name: if is_root { "flask".to_string() } else { dir_name.to_string() },
                command: "flask".to_string(),
                args: vec!["run".to_string()],
                working_dir: rel_path.to_string(),
                port: Some(5000),
                watch_paths: vec![".".to_string()],
                build_cmd: None,
                depends_on: Vec::new(),
            });
        } else if main_py.is_file() {
            detected.push(DetectedService {
                name: if is_root { "app".to_string() } else { dir_name.to_string() },
                command: "python".to_string(),
                args: vec!["main.py".to_string()],
                working_dir: rel_path.to_string(),
                port: None,
                watch_paths: vec![".".to_string()],
                build_cmd: None,
                depends_on: Vec::new(),
            });
        } else if app_py.is_file() {
            detected.push(DetectedService {
                name: if is_root { "app".to_string() } else { dir_name.to_string() },
                command: "python".to_string(),
                args: vec!["app.py".to_string()],
                working_dir: rel_path.to_string(),
                port: None,
                watch_paths: vec![".".to_string()],
                build_cmd: None,
                depends_on: Vec::new(),
            });
        }
    }

    // 5. Ruby on Rails
    if dir.join("Gemfile").is_file() && (dir.join("bin/rails").is_file() || dir.join("config/application.rb").is_file()) {
        detected.push(DetectedService {
            name: if is_root { "rails".to_string() } else { dir_name.to_string() },
            command: "bin/rails".to_string(),
            args: vec!["server".to_string()],
            working_dir: rel_path.to_string(),
            port: Some(3000),
            watch_paths: vec!["app".to_string(), "config".to_string()],
            build_cmd: None,
            depends_on: Vec::new(),
        });
    }

    // 6. Elixir / Phoenix
    if dir.join("mix.exs").is_file() {
        let content = fs::read_to_string(dir.join("mix.exs")).unwrap_or_default();
        if content.contains(":phoenix") {
            detected.push(DetectedService {
                name: if is_root { "phoenix".to_string() } else { dir_name.to_string() },
                command: "mix".to_string(),
                args: vec!["phx.server".to_string()],
                working_dir: rel_path.to_string(),
                port: Some(4000),
                watch_paths: vec!["lib".to_string()],
                build_cmd: None,
                depends_on: Vec::new(),
            });
        } else {
            detected.push(DetectedService {
                name: if is_root { "elixir".to_string() } else { dir_name.to_string() },
                command: "mix".to_string(),
                args: vec!["run".to_string(), "--no-halt".to_string()],
                working_dir: rel_path.to_string(),
                port: None,
                watch_paths: vec!["lib".to_string()],
                build_cmd: None,
                depends_on: Vec::new(),
            });
        }
    }

    // 7. PHP / Laravel
    if dir.join("artisan").is_file() {
        detected.push(DetectedService {
            name: if is_root { "laravel".to_string() } else { dir_name.to_string() },
            command: "php".to_string(),
            args: vec!["artisan".to_string(), "serve".to_string()],
            working_dir: rel_path.to_string(),
            port: Some(8000),
            watch_paths: vec!["app".to_string(), "routes".to_string()],
            build_cmd: None,
            depends_on: Vec::new(),
        });
    }

    // 8. Java / Kotlin
    if dir.join("pom.xml").is_file() {
        detected.push(DetectedService {
            name: if is_root { "api".to_string() } else { dir_name.to_string() },
            command: "mvn".to_string(),
            args: vec!["spring-boot:run".to_string()],
            working_dir: rel_path.to_string(),
            port: Some(8080),
            watch_paths: vec!["src".to_string()],
            build_cmd: Some("mvn compile".to_string()),
            depends_on: Vec::new(),
        });
    } else if dir.join("build.gradle").is_file() || dir.join("build.gradle.kts").is_file() {
        let cmd = if dir.join("gradlew").is_file() { "./gradlew" } else { "gradle" };
        detected.push(DetectedService {
            name: if is_root { "api".to_string() } else { dir_name.to_string() },
            command: cmd.to_string(),
            args: vec!["bootRun".to_string()],
            working_dir: rel_path.to_string(),
            port: Some(8080),
            watch_paths: vec!["src".to_string()],
            build_cmd: Some(format!("{} classes", cmd)),
            depends_on: Vec::new(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_dir(prefix: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("{}_{}", prefix, uuid::Uuid::new_v4()));
        let _ = fs::create_dir_all(&path);
        path
    }

    #[test]
    fn test_init_config_starter_fallback() {
        let dir = create_test_dir("devpane_init_starter");
        let (config_path, services) = init_config(&dir, false).expect("init_config should succeed");
        assert!(config_path.exists());
        assert_eq!(services.len(), 2);
        assert_eq!(services[0].0, "web");
        assert_eq!(services[1].0, "worker");

        let yaml = fs::read_to_string(&config_path).unwrap();
        assert!(yaml.contains("restart: on-failure"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_init_config_fullstack_monorepo() {
        let dir = create_test_dir("devpane_init_fullstack");
        let fe_dir = dir.join("frontend");
        let be_dir = dir.join("backend");
        fs::create_dir_all(&fe_dir).unwrap();
        fs::create_dir_all(&be_dir).unwrap();

        fs::write(
            fe_dir.join("package.json"),
            r#"{"name": "fe", "scripts": {"dev": "vite"}, "dependencies": {"vite": "^5.0"}}"#,
        )
        .unwrap();

        fs::write(
            be_dir.join("Cargo.toml"),
            r#"[package]
name = "be"
version = "0.1.0"
"#,
        )
        .unwrap();

        let (config_path, services) = init_config(&dir, false).expect("init_config should succeed");
        assert!(config_path.exists());
        assert_eq!(services.len(), 2);

        let names: Vec<String> = services.iter().map(|(n, _)| n.clone()).collect();
        assert!(names.contains(&"web".to_string()));
        assert!(names.contains(&"api".to_string()));

        let yaml = fs::read_to_string(&config_path).unwrap();
        assert!(yaml.contains("working_dir: ./frontend"));
        assert!(yaml.contains("working_dir: ./backend"));
        assert!(yaml.contains("depends_on:"));
        assert!(yaml.contains("- api"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_init_config_docker_compose_import() {
        let dir = create_test_dir("devpane_init_compose");
        fs::write(
            dir.join("compose.yaml"),
            r#"services:
  web:
    build: ./frontend
    ports:
      - "3000:3000"
    depends_on:
      - db
  db:
    image: postgres:16
    ports:
      - "5432:5432"
"#,
        )
        .unwrap();

        let (config_path, services) =
            init_config_with_opts(&dir, false, true).expect("compose import should succeed");
        assert!(config_path.exists());
        assert_eq!(services.len(), 2);

        let yaml = fs::read_to_string(&config_path).unwrap();
        assert!(yaml.contains("web:"));
        assert!(yaml.contains("db:"));
        assert!(yaml.contains("working_dir: ./frontend"));
        assert!(yaml.contains("command: \"docker\""));
        assert!(yaml.contains("port: 3000"));
        assert!(yaml.contains("port: 5432"));

        let _ = fs::remove_dir_all(&dir);
    }
}

