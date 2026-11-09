use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use devpane::config::ServiceConfig;
use devpane::process::operations::is_same_process;
use devpane::process::platform;
use devpane::process::session::{ProcessSession, ProcessStatus};
use devpane::process::supervisor::{is_same_project_dir, ProcessSupervisor};

#[test]
fn test_start_project_does_not_duplicate_running_services() {
    let mut supervisor = ProcessSupervisor::new().unwrap();
    let project_dir = std::env::current_dir().unwrap();
    let mut services = HashMap::new();
    let cfg = ServiceConfig {
        enabled: true,
        command: "sleep".to_string(),
        args: vec!["10".to_string()],
        cwd: PathBuf::new(),
        env: HashMap::new(),
        depends_on: None,
        port: None,
        health_check: None,
        restart: None,
        watch: None,
        hooks: None,
        build: None,
        profiles: vec![],
    };
    services.insert("test_sleep".to_string(), cfg);

    // First call: starts service
    let res1 = supervisor.start_project(&project_dir, services.clone());
    assert_eq!(res1.len(), 1);
    assert!(res1[0].error.is_none());
    let pid1 = res1[0].session.as_ref().unwrap().pid;

    // Second call: should skip service because it is already running
    let res2 = supervisor.start_project(&project_dir, services.clone());
    assert_eq!(res2.len(), 1);
    assert!(res2[0].error.is_none());
    let pid2 = res2[0].session.as_ref().unwrap().pid;
    assert_eq!(pid1, pid2);

    // Third call from a new supervisor instance
    let mut supervisor2 = ProcessSupervisor::new().unwrap();
    let res3 = supervisor2.start_project(&project_dir, services.clone());
    assert_eq!(res3.len(), 1);
    assert!(res3[0].error.is_none());
    let pid3 = res3[0].session.as_ref().unwrap().pid;

    // Clean up
    let _ = supervisor2.stop("test_sleep");

    assert_eq!(
        pid1, pid3,
        "New supervisor instance should load existing sessions and not restart them!"
    );
}

#[test]
fn test_is_same_project_dir() {
    let current = std::env::current_dir().unwrap();
    assert!(is_same_project_dir(Some(&current), &current));

    let current_with_slash = current.join("");
    assert!(is_same_project_dir(Some(&current_with_slash), &current));

    assert!(is_same_project_dir(None, &current));
}

#[test]
fn test_stopped_child_is_reaped_and_not_defunct() {
    let mut supervisor = ProcessSupervisor::new().unwrap();
    let current = std::env::current_dir().unwrap();
    let session = supervisor
        .start(
            "reap_test".to_string(),
            "sleep".to_string(),
            vec!["5".to_string()],
            current,
            HashMap::new(),
            None,
        )
        .unwrap();
    let pid = session.pid;
    assert!(platform::is_process_active(pid).unwrap_or(false));

    // Stop the service
    supervisor.stop("reap_test").unwrap();

    // The process should no longer be active (and not a zombie)
    assert!(
        !platform::is_process_active(pid).unwrap_or(false),
        "Stopped process should not remain active or defunct"
    );
}

#[test]
fn test_is_same_process() {
    let mut child = std::process::Command::new("sleep")
        .arg("5")
        .spawn()
        .unwrap();
    let pid = child.id();
    let (s, us) = platform::get_process_start_time(pid).unwrap();
    let session = ProcessSession {
        id: "test".to_string(),
        pid,
        name: "test".to_string(),
        command: "sleep".to_string(),
        start_time_seconds: s,
        start_time_useconds: us,
        cwd: PathBuf::new(),
        status: ProcessStatus::Running,
        exit_code: None,
        project_dir: None,
        cpu_percent: None,
        memory_bytes: None,
    };
    std::thread::sleep(Duration::from_millis(50));
    let matches = is_same_process(&session).unwrap();
    let _ = child.kill();
    assert!(
        matches,
        "is_same_process should return true for running child"
    );
}
