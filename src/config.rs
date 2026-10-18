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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_init_config_package_json() {
        let temp_dir = std::env::temp_dir().join(format!("devpane_test_init_pkg_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        fs::write(
            temp_dir.join("package.json"),
            r#"{"scripts": {"dev": "vite", "build": "vite build"}}"#,
        )
        .unwrap();

        let (path, services) = init_config(&temp_dir, false).unwrap();
        assert!(path.exists());
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].0, "web");

        // Overwrite without force fails
        assert!(init_config(&temp_dir, false).is_err());
        // Overwrite with force succeeds
        assert!(init_config(&temp_dir, true).is_ok());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_init_config_procfile() {
        let temp_dir = std::env::temp_dir().join(format!("devpane_test_init_proc_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        fs::write(
            temp_dir.join("Procfile"),
            "web: bundle exec rails s\nworker: bundle exec sidekiq\n",
        )
        .unwrap();

        let (path, services) = init_config(&temp_dir, false).unwrap();
        assert!(path.exists());
        assert_eq!(services.len(), 2);
        assert_eq!(services[0].0, "web");
        assert_eq!(services[1].0, "worker");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
