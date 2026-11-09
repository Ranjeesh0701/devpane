use std::path::PathBuf;
use std::time::Duration;

use devpane::process::operations::is_same_process;
use devpane::process::platform;
use devpane::process::session::{ProcessSession, ProcessStatus};

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
