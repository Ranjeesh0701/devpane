use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::Duration;

use super::operations::{is_same_process, start_process, stop_process};
use super::session::{ProcessSession, ProcessStatus};
use crate::config::ServiceConfig;
use crate::error::DevboxError;
use crate::protocol::{ServiceStartResult, ServiceStopResult};
use crate::storage::{get_log_file, get_sessions, save_sessions};

struct ManagedProcess {
    child: Option<Child>,
    session: ProcessSession,
}

pub struct ProcessSupervisor {
    processes: Vec<ManagedProcess>,
}

impl ProcessSupervisor {
    pub fn new() -> Result<Self, DevboxError> {
        let sessions = get_sessions()?;

        let processes = sessions
            .into_iter()
            .map(|session| ManagedProcess {
                child: None,
                session,
            })
            .collect();

        Ok(Self { processes })
    }

    fn find_process(&self, id: &str) -> Option<&ManagedProcess> {
        self.processes.iter().find(|p| p.session.id == id)
    }

    pub fn list(&mut self) -> Result<Vec<ProcessSession>, DevboxError> {
        for process in &mut self.processes {
            let child_result = match process.child.as_mut() {
                Some(child) => Some(child.try_wait()),
                None => None,
            };

            match child_result {
                Some(Ok(Some(exit_status))) => {
                    process.session.status = ProcessStatus::Exited;
                    process.session.exit_code = exit_status.code();
                    process.child = None;
                }
                Some(Ok(None)) => {
                    process.session.status = ProcessStatus::Running;
                }
                Some(Err(error)) => {
                    eprintln!(
                        "Failed to check child process {}: {error}",
                        process.session.pid
                    );
                }
                None => match is_same_process(&process.session) {
                    Ok(true) => {
                        process.session.status = ProcessStatus::Running;
                    }
                    Ok(false) => {
                        process.session.status = ProcessStatus::Exited;
                    }
                    Err(error) => {
                        eprintln!("Failed to check process {}: {error}", process.session.pid);
                    }
                },
            }
        }

        self.save_sessions()?;

        Ok(self
            .processes
            .iter()
            .map(|process| process.session.clone())
            .collect())
    }

    pub fn start(
        &mut self,
        name: String,
        program: String,
        args: Vec<String>,
        cwd: PathBuf,
        env: HashMap<String, String>,
        project_dir: Option<PathBuf>,
    ) -> Result<&ProcessSession, DevboxError> {
        let (child, session) = start_process(name, program, args, cwd, env, project_dir)?;
        let session_id = session.id.clone();
        self.processes.push(ManagedProcess {
            child: Some(child),
            session,
        });

        if let Err(save_error) = self.save_sessions() {
            if let Err(stop_error) = self.stop(&session_id) {
                eprintln!("Failed to stop process after save failure: {stop_error:?}");
            }

            return Err(save_error);
        }

        Ok(&self.processes.last().unwrap().session)
    }

    pub fn stop(&mut self, id: &str) -> Result<(), DevboxError> {
        let Some(pos) = self.processes.iter().position(|p| p.session.id == id) else {
            return Err(DevboxError::ProcessNotFound(io::Error::new(
                io::ErrorKind::NotFound,
                "Process not found",
            )));
        };

        let mut managed = self.processes.remove(pos);
        let pid = managed.session.pid;
        let name = managed.session.name.clone();

        match is_same_process(&managed.session) {
            Ok(true) => match stop_process(pid) {
                Ok(true) => {
                    println!("Sent termination signal to {} (PID {})", name, pid);
                }
                Ok(false) => {
                    println!("Process {} (PID {}) is already gone", name, pid);
                }
                Err(e) => return Err(DevboxError::ProcessCheck(e)),
            },
            Ok(false) => {
                println!("Process {} (PID {}) has already exited", name, pid);
            }
            Err(e) => {
                eprintln!(
                    "Warning: could not verify process status for {} (PID {}): {}",
                    name, pid, e
                );
            }
        }

        if let Some(mut child) = managed.child.take() {
            let _ = child.try_wait();
        }

        self.save_sessions()?;

        Ok(())
    }

    pub fn is_running(&mut self, id: &str) -> Result<bool, DevboxError> {
        let sessions = self.list()?;

        Ok(sessions
            .iter()
            .any(|session| session.id == id && session.status == ProcessStatus::Running))
    }

    pub fn start_project(
        &mut self,
        project_dir: &Path,
        services: HashMap<String, ServiceConfig>,
    ) -> Vec<ServiceStartResult> {
        if let Err(error) = self.list() {
            return vec![ServiceStartResult {
                name: "*".to_string(),
                session: None,
                error: Some(format!("Failed to refresh process sessions: {error}")),
            }];
        }

        // Clean up any old exited sessions for this project before starting
        self.processes.retain(|process| {
            !(process.session.project_dir.as_deref() == Some(project_dir)
                && process.session.status == ProcessStatus::Exited)
        });
        let _ = self.save_sessions();

        // Index running services belonging to this project.
        let existing_services: HashMap<String, ProcessSession> = self
            .processes
            .iter()
            .filter(|process| {
                process.session.project_dir.as_deref() == Some(project_dir)
                    && process.session.status == ProcessStatus::Running
            })
            .map(|process| (process.session.name.clone(), process.session.clone()))
            .collect();

        let execution_order = match sort_services_by_dependencies(&services) {
            Ok(order) => order,
            Err(err) => {
                return vec![ServiceStartResult {
                    name: "*".to_string(),
                    session: None,
                    error: Some(err),
                }];
            }
        };

        let mut results = Vec::new();

        for name in execution_order {
            let service = services.get(&name).unwrap();

            // Skip services that are already running.
            if let Some(session) = existing_services.get(&name) {
                results.push(ServiceStartResult {
                    name,
                    session: Some(session.clone()),
                    error: None,
                });
                continue;
            }

            // Smart Port Conflict Resolution: If port is occupied, free it
            let target_port = service
                .port
                .or_else(|| service.health_check.as_ref().and_then(|h| h.port));
            if let Some(port) = target_port {
                if let Err(e) = crate::port::free_port_if_occupied(port) {
                    eprintln!("Warning: failed to check port {port} for service '{name}': {e}");
                }
            }

            match self.start(
                name.clone(),
                service.command.clone(),
                service.args.clone(),
                service.cwd.clone(),
                service.env.clone(),
                Some(project_dir.to_path_buf()),
            ) {
                Ok(session) => {
                    let session_clone = session.clone();

                    // Health check waiting before launching dependent services
                    if let Some(ref hc) = service.health_check {
                        let timeout = Duration::from_secs(hc.timeout_secs);
                        let interval = Duration::from_millis(hc.interval_ms);
                        let check_port = hc.port.or(service.port);
                        if let Err(hc_err) = crate::port::wait_for_health(
                            check_port,
                            hc.http.as_deref(),
                            timeout,
                            interval,
                        ) {
                            eprintln!(
                                "Warning: health check for service '{name}' did not succeed: {hc_err}"
                            );
                        }
                    } else if let Some(port) = service.port {
                        let timeout = Duration::from_secs(10);
                        let interval = Duration::from_millis(150);
                        let _ = crate::port::wait_for_health(Some(port), None, timeout, interval);
                    }

                    results.push(ServiceStartResult {
                        name,
                        session: Some(session_clone),
                        error: None,
                    });
                }
                Err(error) => {
                    results.push(ServiceStartResult {
                        name,
                        session: None,
                        error: Some(error.to_string()),
                    });
                }
            }
        }

        results
    }

    pub fn stop_project(
        &mut self,
        project_dir: &Path,
    ) -> Result<Vec<ServiceStopResult>, DevboxError> {
        let sessions = self.list()?;

        let filtered_sessions: Vec<_> = sessions
            .into_iter()
            .filter(|session| session.project_dir.as_deref() == Some(project_dir))
            .collect();

        let mut results = Vec::new();

        for session in filtered_sessions {
            match self.stop(&session.id) {
                Ok(()) => results.push(ServiceStopResult {
                    name: session.name,
                    error: None,
                }),
                Err(error) => results.push(ServiceStopResult {
                    name: session.name,
                    error: Some(error.to_string()),
                }),
            }
        }

        Ok(results)
    }

    pub fn log_file(&self, id: &str) -> Result<std::path::PathBuf, DevboxError> {
        self.find_process(id)
            .map(|process| get_log_file(&process.session.id))
            .ok_or_else(|| {
                DevboxError::ProcessNotFound(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Session not found",
                ))
            })?
    }

    fn save_sessions(&self) -> Result<(), DevboxError> {
        let sessions: Vec<ProcessSession> =
            self.processes.iter().map(|p| p.session.clone()).collect();

        save_sessions(&sessions)?;

        Ok(())
    }
}

/// Sort services in dependency order using topological sorting.
pub fn sort_services_by_dependencies(
    services: &HashMap<String, ServiceConfig>,
) -> Result<Vec<String>, String> {
    let mut order = Vec::new();
    let mut visited: HashMap<String, bool> = HashMap::new(); // false = visiting (in stack), true = visited

    fn visit(
        name: &str,
        services: &HashMap<String, ServiceConfig>,
        visited: &mut HashMap<String, bool>,
        order: &mut Vec<String>,
    ) -> Result<(), String> {
        match visited.get(name) {
            Some(true) => return Ok(()),
            Some(false) => {
                return Err(format!(
                    "Cyclic dependency detected involving service '{}'",
                    name
                ));
            }
            None => {}
        }

        visited.insert(name.to_string(), false);

        if let Some(config) = services.get(name) {
            for dep in &config.depends_on {
                if !services.contains_key(dep) {
                    return Err(format!(
                        "Service '{}' depends on unknown service '{}'",
                        name, dep
                    ));
                }
                visit(dep, services, visited, order)?;
            }
        }

        visited.insert(name.to_string(), true);
        order.push(name.to_string());
        Ok(())
    }

    let mut names: Vec<_> = services.keys().cloned().collect();
    names.sort(); // Deterministic ordering

    for name in names {
        if !visited.contains_key(&name) {
            visit(&name, services, &mut visited, &mut order)?;
        }
    }

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_service(depends_on: Vec<&str>) -> ServiceConfig {
        ServiceConfig {
            enabled: true,
            command: "echo".to_string(),
            args: vec![],
            cwd: PathBuf::from("."),
            env: HashMap::new(),
            depends_on: depends_on.into_iter().map(String::from).collect(),
            port: None,
            health_check: None,
        }
    }

    #[test]
    fn test_topological_sort_linear() {
        let mut services = HashMap::new();
        services.insert("db".to_string(), dummy_service(vec![]));
        services.insert("api".to_string(), dummy_service(vec!["db"]));
        services.insert("frontend".to_string(), dummy_service(vec!["api"]));

        let order = sort_services_by_dependencies(&services).unwrap();
        let db_idx = order.iter().position(|s| s == "db").unwrap();
        let api_idx = order.iter().position(|s| s == "api").unwrap();
        let fe_idx = order.iter().position(|s| s == "frontend").unwrap();

        assert!(db_idx < api_idx);
        assert!(api_idx < fe_idx);
    }

    #[test]
    fn test_topological_sort_cycle_error() {
        let mut services = HashMap::new();
        services.insert("a".to_string(), dummy_service(vec!["b"]));
        services.insert("b".to_string(), dummy_service(vec!["a"]));

        let result = sort_services_by_dependencies(&services);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cyclic dependency"));
    }

    #[test]
    fn test_topological_sort_missing_dependency() {
        let mut services = HashMap::new();
        services.insert("api".to_string(), dummy_service(vec!["redis"]));

        let result = sort_services_by_dependencies(&services);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("depends on unknown service 'redis'"));
    }
}

