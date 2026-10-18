use std::io;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

fn main() -> io::Result<()> {
    let shutdown = Arc::new(AtomicBool::new(false));
    devpane::daemon::run_daemon(shutdown)
}
