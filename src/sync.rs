use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{find_config_in_dir, load_config};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncReport {
    pub env_created: bool,
    pub missing_env_keys: Vec<String>,
    pub added_env_keys: Vec<String>,
    pub dependencies_installed: Vec<String>,
    pub lockfile_updated: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub has_env: bool,
    pub has_env_example: bool,
    pub missing_env_keys: Vec<String>,
    pub lockfile_exists: bool,
    pub lockfile_in_sync: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockfileServiceEntry {
    pub command: String,
    pub working_dir: String,
    pub port: Option<u16>,
    pub profiles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevpaneLockfile {
    pub version: u32,
    pub config_hash: String,
    pub services: HashMap<String, LockfileServiceEntry>,
}

/// Parses KEY=VALUE or KEY lines from an environment file content.
fn parse_env_file_keys(content: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            entries.push((k.trim().to_string(), v.trim().to_string()));
        } else {
            entries.push((trimmed.to_string(), String::new()));
        }
    }
    entries
}

/// Compute a fast string hash for fingerprinting configuration content.
fn compute_hash(content: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Synchronizes project environment files, dependencies, and lockfile.
pub fn sync_project(
    dir: &Path,
    install_deps: bool,
) -> Result<SyncReport, Box<dyn error::Error>> {
    let mut report = SyncReport {
        env_created: false,
        missing_env_keys: Vec::new(),
        added_env_keys: Vec::new(),
        dependencies_installed: Vec::new(),
        lockfile_updated: false,
        warnings: Vec::new(),
    };

    // 1. Sync .env from .env.example
    let env_example_path = dir.join(".env.example");
    let env_path = dir.join(".env");

    if env_example_path.is_file() {
        if !env_path.exists() {
            if let Ok(_) = fs::copy(&env_example_path, &env_path) {
                report.env_created = true;
            } else {
                report.warnings.push("Failed to copy .env.example to .env".to_string());
            }
        } else if let Ok(example_content) = fs::read_to_string(&env_example_path) {
            let env_content = fs::read_to_string(&env_path).unwrap_or_default();
            let existing_keys: HashSet<String> = parse_env_file_keys(&env_content)
                .into_iter()
                .map(|(k, _)| k)
                .collect();

            let example_entries = parse_env_file_keys(&example_content);
            let mut missing_to_append = Vec::new();

            for (key, val) in example_entries {
                if !existing_keys.contains(&key) {
                    report.missing_env_keys.push(key.clone());
                    missing_to_append.push((key, val));
                }
            }

            if !missing_to_append.is_empty() {
                let mut append_content = String::new();
                append_content.push_str("\n# Missing keys synced from .env.example by devpane sync\n");
                for (k, v) in &missing_to_append {
                    append_content.push_str(&format!("{}={}\n", k, v));
                    report.added_env_keys.push(k.clone());
                }
                let mut full = env_content;
                full.push_str(&append_content);
                if let Err(e) = fs::write(&env_path, full) {
                    report.warnings.push(format!("Failed to append missing keys to .env: {e}"));
                }
            }
        }
    }

    // 2. Load devpane.yaml configuration
    let config_path = match find_config_in_dir(dir) {
        Ok(p) => p,
        Err(_) => {
            report.warnings.push("No devpane.yaml found to sync services".to_string());
            return Ok(report);
        }
    };

    let config = load_config(&config_path)?;
    let config_raw = fs::read_to_string(&config_path).unwrap_or_default();
    let current_hash = compute_hash(&config_raw);

    // 3. Install dependencies across services if requested
    if install_deps {
        let mut processed_cwds: HashSet<PathBuf> = HashSet::new();

        for (name, service) in &config.services {
            let cwd = if service.cwd.is_absolute() {
                service.cwd.clone()
            } else {
                dir.join(&service.cwd)
            };

            // Avoid running install multiple times in the same working directory
            if !processed_cwds.insert(cwd.clone()) {
                continue;
            }

            if cwd.join("package.json").is_file() {
                let pkg_mgr = if cwd.join("bun.lock").exists() || cwd.join("bun.lockb").exists() {
                    "bun"
                } else if cwd.join("pnpm-lock.yaml").exists() {
                    "pnpm"
                } else if cwd.join("yarn.lock").exists() {
                    "yarn"
                } else {
                    "npm"
                };

                let status = Command::new(pkg_mgr)
                    .arg("install")
                    .current_dir(&cwd)
                    .status();

                match status {
                    Ok(s) if s.success() => {
                        report.dependencies_installed.push(format!("{}: {} install", name, pkg_mgr));
                    }
                    Ok(s) => {
                        report.warnings.push(format!("{}: {} install exited with code {:?}", name, pkg_mgr, s.code()));
                    }
                    Err(e) => {
                        report.warnings.push(format!("{}: failed to run {} install: {e}", name, pkg_mgr));
                    }
                }
            } else if cwd.join("Cargo.toml").is_file() {
                let status = Command::new("cargo")
                    .arg("check")
                    .current_dir(&cwd)
                    .status();

                match status {
                    Ok(s) if s.success() => {
                        report.dependencies_installed.push(format!("{}: cargo check", name));
                    }
                    Ok(s) => {
                        report.warnings.push(format!("{}: cargo check exited with code {:?}", name, s.code()));
                    }
                    Err(e) => {
                        report.warnings.push(format!("{}: failed to run cargo check: {e}", name));
                    }
                }
            } else if cwd.join("go.mod").is_file() {
                let status = Command::new("go")
                    .args(["mod", "download"])
                    .current_dir(&cwd)
                    .status();

                match status {
                    Ok(s) if s.success() => {
                        report.dependencies_installed.push(format!("{}: go mod download", name));
                    }
                    Ok(s) => {
                        report.warnings.push(format!("{}: go mod download exited with code {:?}", name, s.code()));
                    }
                    Err(e) => {
                        report.warnings.push(format!("{}: failed to run go mod download: {e}", name));
                    }
                }
            } else if cwd.join("requirements.txt").is_file() {
                let status = Command::new("pip")
                    .args(["install", "-r", "requirements.txt"])
                    .current_dir(&cwd)
                    .status();

                match status {
                    Ok(s) if s.success() => {
                        report.dependencies_installed.push(format!("{}: pip install -r requirements.txt", name));
                    }
                    Ok(s) => {
                        report.warnings.push(format!("{}: pip install exited with code {:?}", name, s.code()));
                    }
                    Err(e) => {
                        report.warnings.push(format!("{}: failed to run pip install: {e}", name));
                    }
                }
            }
        }
    }

    // 4. Update devpane.lock lockfile
    let lock_path = dir.join("devpane.lock");
    let mut lock_services = HashMap::new();

    for (name, service) in &config.services {
        lock_services.insert(
            name.clone(),
            LockfileServiceEntry {
                command: if service.args.is_empty() {
                    service.command.clone()
                } else {
                    format!("{} {}", service.command, service.args.join(" "))
                },
                working_dir: service.cwd.display().to_string(),
                port: service.port,
                profiles: service.profiles.clone(),
            },
        );
    }

    let lockfile = DevpaneLockfile {
        version: 1,
        config_hash: current_hash,
        services: lock_services,
    };

    let lock_yaml = match serde_yaml_ng::to_string(&lockfile) {
        Ok(y) => format!(
            "# Devpane Team Lockfile\n# Generated by 'devpane sync'. Commit this file to synchronize team environments.\n\n{}",
            y
        ),
        Err(e) => {
            report.warnings.push(format!("Failed to serialize devpane.lock: {e}"));
            return Ok(report);
        }
    };

    let needs_write = match fs::read_to_string(&lock_path) {
        Ok(existing) => existing != lock_yaml,
        Err(_) => true,
    };

    if needs_write {
        if let Err(e) = fs::write(&lock_path, lock_yaml) {
            report.warnings.push(format!("Failed to write devpane.lock: {e}"));
        } else {
            report.lockfile_updated = true;
        }
    }

    Ok(report)
}

/// Checks team sync status without running installs or modifying files.
pub fn check_sync_status(dir: &Path) -> SyncStatus {
    let env_example_path = dir.join(".env.example");
    let env_path = dir.join(".env");
    let lock_path = dir.join("devpane.lock");

    let has_env_example = env_example_path.is_file();
    let has_env = env_path.is_file();
    let lockfile_exists = lock_path.is_file();

    let mut missing_env_keys = Vec::new();
    if has_env_example && has_env {
        if let (Ok(example_content), Ok(env_content)) = (
            fs::read_to_string(&env_example_path),
            fs::read_to_string(&env_path),
        ) {
            let existing: HashSet<String> = parse_env_file_keys(&env_content)
                .into_iter()
                .map(|(k, _)| k)
                .collect();
            for (key, _) in parse_env_file_keys(&example_content) {
                if !existing.contains(&key) {
                    missing_env_keys.push(key);
                }
            }
        }
    }

    let lockfile_in_sync = if lockfile_exists {
        if let Ok(config_path) = find_config_in_dir(dir) {
            if let (Ok(config_raw), Ok(lock_raw)) = (
                fs::read_to_string(&config_path),
                fs::read_to_string(&lock_path),
            ) {
                let current_hash = compute_hash(&config_raw);
                if let Ok(lockfile) = serde_yaml_ng::from_str::<DevpaneLockfile>(&lock_raw) {
                    lockfile.config_hash == current_hash
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            true
        }
    } else {
        false
    };

    SyncStatus {
        has_env,
        has_env_example,
        missing_env_keys,
        lockfile_exists,
        lockfile_in_sync,
    }
}
