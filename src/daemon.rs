use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::ipc;
use crate::process::supervisor::ProcessSupervisor;
use crate::protocol::{read_message, write_message, Request, Response};
use crate::storage::ensure_storage_directory;

pub fn run_daemon(shutdown: Arc<AtomicBool>) -> io::Result<()> {
    let supervisor = Arc::new(Mutex::new(ProcessSupervisor::new().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to initialize supervisor: {}", e),
        )
    })?));
    let mut client_threads = Vec::new();

    let storage_directory = ensure_storage_directory().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to create directory: {}", e),
        )
    })?;
    let temp_file = storage_directory.join(crate::storage::SESSIONS_TMP);

    if temp_file.exists() {
        let _ = fs::remove_file(&temp_file);
    }

    let listener = ipc::prepare_listener()?;

    loop {
        if shutdown.load(Ordering::SeqCst) {
            break;
        }

        match listener.accept() {
            Ok(Some(stream)) => {
                let supervisor = Arc::clone(&supervisor);
                let shutdown_signal = Arc::clone(&shutdown);

                let handle = thread::spawn(move || {
                    if let Err(error) = handle_client(stream, supervisor, shutdown_signal) {
                        eprintln!("Failed to handle client: {error}");
                    }
                });
                client_threads.push(handle);
            }
            Ok(None) => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(error) => {
                eprintln!("Failed to accept connection: {error}");
            }
        }
    }

    for handle in client_threads {
        if let Err(error) = handle.join() {
            eprintln!("Failed to join client thread: {error:?}");
        }
    }

    ipc::remove_socket()?;
    Ok(())
}

fn handle_client(
    mut stream: ipc::Connection,
    supervisor: Arc<Mutex<ProcessSupervisor>>,
    shutdown: Arc<AtomicBool>,
) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let request: Request = read_message(&mut stream)?;
    stream.set_read_timeout(None)?;

    if shutdown.load(Ordering::SeqCst) {
        write_message(
            &mut stream,
            &Response::Error {
                message: "Daemon is shutting down".to_string(),
            },
        )?;
        return Ok(());
    }

    if let Request::Logs { id, follow } = &request {
        let result = {
            let supervisor = supervisor
                .lock()
                .map_err(|_| io::Error::other("Supervisor lock poisoned"))?;

            supervisor.log_file(&id)
        };

        match result {
            Ok(ref path) => {
                if let Err(error) = logs(
                    &mut stream,
                    path,
                    *follow,
                    || {
                        let mut supervisor = supervisor
                            .lock()
                            .map_err(|_| io::Error::other("Supervisor lock poisoned"))?;

                        supervisor
                            .is_running(id)
                            .map_err(|error| io::Error::other(error.to_string()))
                    },
                    || shutdown.load(Ordering::SeqCst),
                ) {
                    if error.kind() != io::ErrorKind::BrokenPipe
                        && error.kind() != io::ErrorKind::ConnectionReset
                    {
                        eprintln!("Failed to stream logs: {error}");
                    }
                }
            }
            Err(error) => {
                write_message(
                    &mut stream,
                    &Response::Error {
                        message: format!("Failed to get logs: {error}"),
                    },
                )?;
            }
        }

        return Ok(());
    }

    let is_shutdown = matches!(&request, Request::Shutdown);

    let response: Response = match supervisor.lock() {
        Err(_) => Response::Error {
            message: "Supervisor lock poisoned".to_string(),
        },
        Ok(mut supervisor) => match request {
            Request::Start {
                name,
                program,
                args,
                cwd,
                env,
            } => match supervisor.start(name, program, args, cwd, env, None) {
                Ok(session) => Response::Start {
                    session: session.clone(),
                },
                Err(e) => Response::Error {
                    message: format!("Failed to start: {e}"),
                },
            },
            Request::List => match supervisor.list() {
                Ok(sessions) => Response::List { sessions },
                Err(e) => Response::Error {
                    message: format!("Failed to list: {e}"),
                },
            },
            Request::Stop { id } => match supervisor.stop(&id) {
                Ok(()) => Response::Stop,
                Err(e) => Response::Error {
                    message: format!("Failed to stop: {e}"),
                },
            },
            Request::Logs { .. } => unreachable!("Logs requests are handled above"),
            Request::Shutdown => {
                Response::Shutdown
            }
            Request::Up {
                project_dir,
                services,
            } => Response::Up {
                results: supervisor.start_project(&project_dir, services),
            },
            Request::Down { project_dir } => match supervisor.stop_project(&project_dir) {
                Ok(results) => Response::Down { results },
                Err(error) => Response::Error {
                    message: format!("Failed to stop project: {error}"),
                },
            },
        },
    };

    write_message(&mut stream, &response)?;

    if is_shutdown {
        shutdown.store(true, Ordering::SeqCst);
    }

    Ok(())
}

fn logs<F, S>(
    stream: &mut ipc::Connection,
    path: &Path,
    follow: bool,
    mut is_running: F,
    mut should_shutdown: S,
) -> io::Result<()>
where
    F: FnMut() -> io::Result<bool>,
    S: FnMut() -> bool,
{
    let mut file = fs::File::open(path)?;
    let mut buffer = [0u8; 4096];

    loop {
        let bytes_read = file.read(&mut buffer)?;

        if bytes_read > 0 {
            let logs = String::from_utf8_lossy(&buffer[..bytes_read]).into_owned();
            write_message(stream, &Response::LogChunk { logs })?;
        } else if !follow || !is_running()? || should_shutdown() {
            break;
        } else {
            thread::sleep(Duration::from_millis(200));
        }
    }

    write_message(stream, &Response::LogStreamEnd)
}
