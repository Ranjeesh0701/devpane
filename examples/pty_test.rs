use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use devpane::terminal::TerminalSession;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = TerminalSession::spawn(
        "/bin/sh",
        &[
            "-c".to_string(),
            "sleep 60 & echo CHILD_PID:$!; wait".to_string(),
        ],
        Path::new("."),
        24,
        80,
    )?;

    let deadline = Instant::now() + Duration::from_secs(2);
    let mut output = String::new();

    while Instant::now() < deadline {
        match terminal.try_read_output() {
            Ok(ref bytes) => {
                let text = String::from_utf8_lossy(bytes);
                print!("{text}");
                output.push_str(&text);

                if output.contains("CHILD_PID:") {
                    break;
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                break;
            }
        }
    }

    let pid = output
        .split("CHILD_PID:")
        .nth(1)
        .and_then(|value| value.split_whitespace().next())
        .and_then(|value| value.parse::<i32>().ok())
        .ok_or("Could not capture the background process PID")?;

    println!("Background sleep PID: {pid}");
    println!("Terminating shell...");
    terminal.terminate()?;

    for _ in 0..20 {
        if let Some(status) = terminal.try_wait()? {
            println!("Shell exited: {status:?}");
            break;
        }

        thread::sleep(Duration::from_millis(100));
    }

    // Check whether the background process still exists.
    let result = unsafe { libc::kill(pid, 0) };

    if result == 0 {
        println!("Background process still exists.");
    } else {
        let error = std::io::Error::last_os_error();

        if error.raw_os_error() == Some(libc::ESRCH) {
            println!("Background process no longer exists.");
        } else {
            println!("Could not determine background process state: {error}");
        }
    }

    Ok(())
}
