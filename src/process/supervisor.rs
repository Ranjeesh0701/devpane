use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::{Duration, Instant};

use super::operations::{is_same_process, start_process, stop_process};
use super::session::{ProcessSession, ProcessStatus};
use crate::config::{DependencyCondition, RestartMode, ServiceConfig, WatchConfig};
use crate::error::DevpaneError;
use crate::protocol::{ServiceStartResult, ServiceStopResult};
use crate::storage::{get_log_file, get_sessions, save_sessions};
use crate::watcher::ServiceWatcher;

struct ManagedProcess {
    child: Option<Child>,
    session: ProcessSession,
}

#[derive(Clone)]
struct ServiceMeta {
    config: ServiceConfig,
    project_dir: PathBuf,
    crash_count: u32,
    last_crash: Option<Instant>,
    next_restart_at: Option<Instant>,
}

pub struct ProcessSupervisor {
    processes: Vec<ManagedProcess>,
    services_meta: HashMap<String, ServiceMeta>,
    watcher: Option<ServiceWatcher>,
}

impl ProcessSupervisor {
    pub fn new() -> Result<Self, DevpaneError> {
        let sessions = get_sessions()?;

        let processes = sessions
            .into_iter()
            .map(|session| ManagedProcess {
                child: None,
                session,
            })
            .collect();

        let watcher = ServiceWatcher::new().ok();

        Ok(Self {
            processes,
            services_meta: HashMap::new(),
            watcher,
        })
    }

    fn find_process(&self, id: &str) -> Option<&ManagedProcess> {
        self.processes.iter().find(|p| p.session.id == id)
    }

    pub fn list(&mut self) -> Result<Vec<ProcessSession>, DevpaneError> {
        for process in &mut self.processes {
            let child_result = match process.child.as_mut() {
                Some(child) => Some(child.try_wait()),
                None => None,
            };

            match child_result {
                Some(Ok(Some(exit_status))) => {
                    process.session.status = ProcessStatus::Exited;
                    process.session.exit_code = exit_status.code();
                    process.session.cpu_percent = None;
                    process.session.memory_bytes = None;
                    process.child = None;
                }
                Some(Ok(None)) => {
                    process.session.status = ProcessStatus::Running;
                    let (cpu, mem) = super::platform::get_process_metrics(process.session.pid);
                    process.session.cpu_percent = cpu;
                    process.session.memory_bytes = mem;
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
                        let (cpu, mem) = super::platform::get_process_metrics(process.session.pid);
                        process.session.cpu_percent = cpu;
                        process.session.memory_bytes = mem;
                    }
                    Ok(false) => {
                        process.session.status = ProcessStatus::Exited;
                        process.session.cpu_percent = None;
                        process.session.memory_bytes = None;
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
    ) -> Result<&ProcessSession, DevpaneError> {
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

    pub fn stop(&mut self, id: &str) -> Result<(), DevpaneError> {
        let Some(pos) = self.processes.iter().position(|p| p.session.id == id) else {
            return Err(DevpaneError::ProcessNotFound(io::Error::new(
                io::ErrorKind::NotFound,
                "Process not found",
            )));
        };

        let mut managed = self.processes.remove(pos);
        let pid = managed.session.pid;
        let name = managed.session.name.clone();

        // Run pre_stop hooks if defined
        if let Some(meta) = self.services_meta.get(&name) {
            if let Some(ref hooks) = meta.config.hooks {
                if let Some(ref pre_stop) = hooks.pre_stop {
                    let cwd = if meta.config.cwd.is_absolute() {
                        meta.config.cwd.clone()
                    } else {
                        meta.project_dir.join(&meta.config.cwd)
                    };
                    let _ = super::hooks::run_command_list(&pre_stop.to_vec(), &cwd, &meta.config.env, None);
                }
            }
        }

        match is_same_process(&managed.session) {
            Ok(true) => match stop_process(pid) {
                Ok(true) => {
                    println!("Sent termination signal to {} (PID {})", name, pid);
                }
                Ok(false) => {
                    println!("Process {} (PID {}) is already gone", name, pid);
                }
                Err(e) => return Err(DevpaneError::ProcessCheck(e)),
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

        // Run post_stop hooks if defined
        if let Some(meta) = self.services_meta.get(&name) {
            if let Some(ref hooks) = meta.config.hooks {
                if let Some(ref post_stop) = hooks.post_stop {
                    let cwd = if meta.config.cwd.is_absolute() {
                        meta.config.cwd.clone()
                    } else {
                        meta.project_dir.join(&meta.config.cwd)
                    };
                    let _ = super::hooks::run_command_list(&post_stop.to_vec(), &cwd, &meta.config.env, None);
                }
            }
        }

        // Unwatch if registered in watcher
        if let Some(ref mut watcher) = self.watcher {
            watcher.unwatch_service(&name);
        }

        self.save_sessions()?;

        Ok(())
    }

    pub fn is_running(&mut self, id: &str) -> Result<bool, DevpaneError> {
        let sessions = self.list()?;

        Ok(sessions
            .iter()
            .any(|session| session.id == id && session.status == ProcessStatus::Running))
    }

    pub fn start_service_internal(
        &mut self,
        name: &str,
        service: &ServiceConfig,
        project_dir: &Path,
    ) -> Result<ProcessSession, DevpaneError> {
        let cwd = if service.cwd.is_absolute() {
            service.cwd.clone()
        } else {
            project_dir.join(&service.cwd)
        };

        let env_files = crate::config::load_env_files(project_dir);
        let mut final_env = env_files;
        final_env.extend(service.env.clone());

        let log_path = get_log_file(name).ok();

        // 1. Pre-build hooks
        if let Some(ref hooks) = service.hooks {
            if let Some(ref pre_build) = hooks.pre_build {
                super::hooks::run_command_list(
                    &pre_build.to_vec(),
                    &cwd,
                    &final_env,
                    log_path.as_deref(),
                )
                .map_err(|e| DevpaneError::InvalidArguments(format!("Pre-build hook failed for '{name}': {e}")))?;
            }
        }

        // 2. Build Pipeline
        if let Some(ref build) = service.build {
            let build_cwd = build
                .cwd
                .as_ref()
                .map(|c| {
                    if c.is_absolute() {
                        c.clone()
                    } else {
                        project_dir.join(c)
                    }
                })
                .unwrap_or_else(|| cwd.clone());

            let should_build = if let Some(ref check) = build.check {
                super::hooks::check_build_required(
                    &check.to_vec().join(" "),
                    &build_cwd,
                    &final_env,
                )
            } else {
                true
            };

            if should_build {
                if let Some(ref cmd) = build.cmd {
                    super::hooks::run_command_list(
                        &cmd.to_vec(),
                        &build_cwd,
                        &final_env,
                        log_path.as_deref(),
                    )
                    .map_err(|e| DevpaneError::InvalidArguments(format!("Build command failed for '{name}': {e}")))?;
                }
            }
        }

        // 3. Post-build hooks
        if let Some(ref hooks) = service.hooks {
            if let Some(ref post_build) = hooks.post_build {
                super::hooks::run_command_list(
                    &post_build.to_vec(),
                    &cwd,
                    &final_env,
                    log_path.as_deref(),
                )
                .map_err(|e| DevpaneError::InvalidArguments(format!("Post-build hook failed for '{name}': {e}")))?;
            }
        }

        // 4. Smart Port Conflict Resolution: If port is occupied, free it
        let target_port = service
            .port
            .or_else(|| service.health_check.as_ref().and_then(|h| h.port));
        if let Some(port) = target_port {
            if let Err(e) = crate::port::free_port_if_occupied(port) {
                eprintln!("Warning: failed to check port {port} for service '{name}': {e}");
            }
        }

        // 5. Pre-start hooks
        if let Some(ref hooks) = service.hooks {
            if let Some(ref pre_start) = hooks.pre_start {
                super::hooks::run_command_list(
                    &pre_start.to_vec(),
                    &cwd,
                    &final_env,
                    log_path.as_deref(),
                )
                .map_err(|e| DevpaneError::InvalidArguments(format!("Pre-start hook failed for '{name}': {e}")))?;
            }
        }

        // 6. Launch service process
        let session = self.start(
            name.to_string(),
            service.command.clone(),
            service.args.clone(),
            cwd.clone(),
            final_env.clone(),
            Some(project_dir.to_path_buf()),
        )?
        .clone();

        // 7. Post-start hooks
        if let Some(ref hooks) = service.hooks {
            if let Some(ref post_start) = hooks.post_start {
                let _ = super::hooks::run_command_list(
                    &post_start.to_vec(),
                    &cwd,
                    &final_env,
                    log_path.as_deref(),
                );
            }
        }

        // 8. Health check waiting before launching dependent services
        if let Some(ref hc) = service.health_check {
            let timeout = Duration::from_secs(hc.timeout_secs);
            let interval = Duration::from_millis(hc.interval_ms);
            let check_port = hc.port.or(service.port);
            let check_cmd = hc
                .command
                .as_ref()
                .and_then(|c| c.to_vec().into_iter().next());

            if let Err(hc_err) = crate::port::wait_for_health_full(
                check_port,
                hc.http.as_deref(),
                check_cmd.as_deref(),
                Some(&cwd),
                Some(&final_env),
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

        // 9. Register file watch if configured or on-change restart mode
        if let Some(watch_cfg) = service.watch_config() {
            if let Some(ref mut watcher) = self.watcher {
                if let Err(e) = watcher.watch_service(name, &cwd, &watch_cfg) {
                    eprintln!("Warning: failed to watch service '{name}': {e}");
                }
            }
        } else if service
            .restart_config()
            .map(|r| r.on == RestartMode::Change)
            .unwrap_or(false)
        {
            if let Some(ref mut watcher) = self.watcher {
                let default_watch = WatchConfig {
                    include: vec![".".to_string()],
                    ..Default::default()
                };
                let _ = watcher.watch_service(name, &cwd, &default_watch);
            }
        }

        Ok(session)
    }

    pub fn register_service(&mut self, name: &str, service: &ServiceConfig, project_dir: &Path) {
        self.services_meta.insert(
            name.to_string(),
            ServiceMeta {
                config: service.clone(),
                project_dir: project_dir.to_path_buf(),
                crash_count: 0,
                last_crash: None,
                next_restart_at: None,
            },
        );
    }

    pub fn restart_service(&mut self, name_or_id: &str) -> Result<ProcessSession, DevpaneError> {
        let (resolved_name, session_id) = if self.services_meta.contains_key(name_or_id) {
            let id = self
                .processes
                .iter()
                .find(|p| p.session.name == name_or_id)
                .map(|p| p.session.id.clone());
            (name_or_id.to_string(), id)
        } else if let Some(p) = self.processes.iter().find(|p| p.session.id == name_or_id || p.session.name == name_or_id) {
            (p.session.name.clone(), Some(p.session.id.clone()))
        } else {
            (name_or_id.to_string(), None)
        };

        if let Some(ref id) = session_id {
            let _ = self.stop(id);
        }

        if let Some(meta) = self.services_meta.get(&resolved_name).cloned() {
            self.start_service_internal(&resolved_name, &meta.config, &meta.project_dir)
        } else {
            Err(DevpaneError::InvalidArguments(format!(
                "No configuration found for service '{name_or_id}'"
            )))
        }
    }

    pub fn rebuild_service(&mut self, name_or_id: &str) -> Result<(), DevpaneError> {
        let (resolved_name, session_id) = if self.services_meta.contains_key(name_or_id) {
            let id = self
                .processes
                .iter()
                .find(|p| p.session.name == name_or_id)
                .map(|p| p.session.id.clone());
            (name_or_id.to_string(), id)
        } else if let Some(p) = self.processes.iter().find(|p| p.session.id == name_or_id || p.session.name == name_or_id) {
            (p.session.name.clone(), Some(p.session.id.clone()))
        } else {
            (name_or_id.to_string(), None)
        };

        if let Some(ref id) = session_id {
            let _ = self.stop(id);
        }

        let meta = self.services_meta.get(&resolved_name).cloned().ok_or_else(|| {
            DevpaneError::InvalidArguments(format!("No configuration found for service '{name_or_id}'"))
        })?;

        let cwd = if meta.config.cwd.is_absolute() {
            meta.config.cwd.clone()
        } else {
            meta.project_dir.join(&meta.config.cwd)
        };

        let env_files = crate::config::load_env_files(&meta.project_dir);
        let mut final_env = env_files;
        final_env.extend(meta.config.env.clone());

        let log_path = get_log_file(&resolved_name).ok();

        // 1. Pre-build
        if let Some(ref hooks) = meta.config.hooks {
            if let Some(ref pre_build) = hooks.pre_build {
                super::hooks::run_command_list(&pre_build.to_vec(), &cwd, &final_env, log_path.as_deref())
                    .map_err(|e| DevpaneError::InvalidArguments(format!("Pre-build hook failed: {e}")))?;
            }
        }

        // 2. Unconditional Build CMD
        if let Some(ref build) = meta.config.build {
            if let Some(ref cmd) = build.cmd {
                let build_cwd = build.cwd.as_ref().map(|c| if c.is_absolute() { c.clone() } else { meta.project_dir.join(c) }).unwrap_or_else(|| cwd.clone());
                super::hooks::run_command_list(&cmd.to_vec(), &build_cwd, &final_env, log_path.as_deref())
                    .map_err(|e| DevpaneError::InvalidArguments(format!("Build command failed: {e}")))?;
            }
        }

        // 3. Post-build
        if let Some(ref hooks) = meta.config.hooks {
            if let Some(ref post_build) = hooks.post_build {
                super::hooks::run_command_list(&post_build.to_vec(), &cwd, &final_env, log_path.as_deref())
                    .map_err(|e| DevpaneError::InvalidArguments(format!("Post-build hook failed: {e}")))?;
            }
        }

        // 4. Start service again
        self.start_service_internal(&resolved_name, &meta.config, &meta.project_dir)?;

        Ok(())
    }

    pub fn restart_project(
        &mut self,
        project_dir: &Path,
        services: HashMap<String, ServiceConfig>,
    ) -> Vec<ServiceStartResult> {
        let _ = self.stop_project(project_dir);

        let svcs = if services.is_empty() {
            self.services_meta
                .iter()
                .filter(|(_, meta)| meta.project_dir == project_dir)
                .map(|(n, meta)| (n.clone(), meta.config.clone()))
                .collect()
        } else {
            services
        };

        self.start_project(project_dir, svcs)
    }

    pub fn tick(&mut self) -> Vec<String> {
        let _ = self.list();
        let mut actions = Vec::new();
        let now = Instant::now();

        // 1. Process file watch triggers
        let changed_services = if let Some(ref mut watcher) = self.watcher {
            watcher.poll_changed_services()
        } else {
            Vec::new()
        };

        for name in changed_services {
            if self.services_meta.contains_key(&name) {
                println!("🔄 File change detected for service '{}', restarting...", name);
                if let Err(e) = self.restart_service(&name) {
                    eprintln!("Failed to restart service '{name}' after file change: {e}");
                } else {
                    actions.push(format!("restarted {name} on file change"));
                }
            }
        }

        // 2. Process crash recovery
        let mut to_restart = Vec::new();
        for (name, meta) in &mut self.services_meta {
            let restart_cfg = match meta.config.restart_config() {
                Some(cfg) => cfg,
                None => continue,
            };

            if !matches!(
                restart_cfg.on,
                RestartMode::Crash | RestartMode::Always
            ) {
                continue;
            }

            // Check current status of the service process
            let current_proc = self.processes.iter().find(|p| p.session.name == *name);
            let has_exited = match current_proc {
                Some(p) => p.session.status == ProcessStatus::Exited,
                None => false,
            };

            if has_exited {
                let exit_code = current_proc.and_then(|p| p.session.exit_code).unwrap_or(0);
                let is_failure = exit_code != 0 || restart_cfg.on == RestartMode::Always;

                if is_failure {
                    if meta.crash_count < restart_cfg.max_crash_retries {
                        let backoff = Duration::from_millis(
                            restart_cfg
                                .backoff_ms
                                .saturating_mul(1 << meta.crash_count.min(4)),
                        );

                        let due = match meta.next_restart_at {
                            Some(t) => t,
                            None => {
                                let t = now + backoff;
                                meta.next_restart_at = Some(t);
                                meta.last_crash = Some(now);
                                t
                            }
                        };

                        if now >= due {
                            meta.crash_count += 1;
                            meta.next_restart_at = None;
                            to_restart.push((
                                name.clone(),
                                meta.crash_count,
                                restart_cfg.max_crash_retries,
                            ));
                        }
                    } else if meta.crash_count == restart_cfg.max_crash_retries {
                        eprintln!(
                            "⚠️  Service '{}' reached maximum crash retries ({}), will not restart.",
                            name, restart_cfg.max_crash_retries
                        );
                        meta.crash_count += 1; // Prevent repeated logs
                    }
                }
            } else if meta.crash_count > 0 {
                if let Some(last_crash) = meta.last_crash {
                    if now.duration_since(last_crash) > Duration::from_secs(30) {
                        meta.crash_count = 0;
                        meta.last_crash = None;
                    }
                }
            }
        }

        for (name, attempt, max) in to_restart {
            println!(
                "⚠️  Restarting crashed service '{}' (attempt {}/{})...",
                name, attempt, max
            );
            if let Err(e) = self.restart_service(&name) {
                eprintln!("Failed to restart service '{name}': {e}");
            } else {
                actions.push(format!("restarted {name} after crash ({attempt}/{max})"));
            }
        }

        actions
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

        // Store / update metadata for all project services
        for (name, service) in &services {
            self.services_meta.insert(
                name.clone(),
                ServiceMeta {
                    config: service.clone(),
                    project_dir: project_dir.to_path_buf(),
                    crash_count: 0,
                    last_crash: None,
                    next_restart_at: None,
                },
            );
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

            // Verify dependency conditions
            for (dep_name, dep_spec) in service.dependency_map() {
                match dep_spec.condition {
                    DependencyCondition::Started => {
                        let started = self.processes.iter().any(|p| p.session.name == dep_name);
                        if !started {
                            eprintln!("Warning: dependency '{dep_name}' was not started");
                        }
                    }
                    DependencyCondition::Running | DependencyCondition::Ready => {
                        let timeout = Duration::from_secs(dep_spec.timeout_secs.unwrap_or(30));
                        let start = Instant::now();
                        let mut ok = false;
                        while start.elapsed() < timeout {
                            if self.processes.iter().any(|p| {
                                p.session.name == dep_name
                                    && p.session.status == ProcessStatus::Running
                            }) {
                                ok = true;
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(150));
                            let _ = self.list();
                        }
                        if !ok {
                            eprintln!("Warning: dependency '{dep_name}' did not meet condition {:?}", dep_spec.condition);
                        }
                    }
                }
            }

            match self.start_service_internal(&name, service, project_dir) {
                Ok(session) => {
                    results.push(ServiceStartResult {
                        name,
                        session: Some(session),
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
    ) -> Result<Vec<ServiceStopResult>, DevpaneError> {
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

    pub fn log_file(&self, id: &str) -> Result<std::path::PathBuf, DevpaneError> {
        self.find_process(id)
            .map(|process| get_log_file(&process.session.id))
            .ok_or_else(|| {
                DevpaneError::ProcessNotFound(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Session not found",
                ))
            })?
    }

    fn save_sessions(&self) -> Result<(), DevpaneError> {
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
            for dep in config.dependency_names() {
                if !services.contains_key(&dep) {
                    return Err(format!(
                        "Service '{}' depends on unknown service '{}'",
                        name, dep
                    ));
                }
                visit(&dep, services, visited, order)?;
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
