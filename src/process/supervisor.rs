use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Child;

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

        let mut results = Vec::new();

        let mut services: Vec<_> = services.into_iter().collect();
        services.sort_by(|a, b| a.0.cmp(&b.0));

        for (name, service) in services {
            // Skip services that are already running.
            if let Some(session) = existing_services.get(&name) {
                results.push(ServiceStartResult {
                    name,
                    session: Some(session.clone()),
                    error: None,
                });
                continue;
            }

            match self.start(
                name.clone(),
                service.command,
                service.args,
                service.cwd,
                service.env,
                Some(project_dir.to_path_buf()),
            ) {
                Ok(session) => {
                    results.push(ServiceStartResult {
                        name,
                        session: Some(session.clone()),
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
