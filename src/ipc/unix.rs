use super::{Connection, ReadWrite};

use std::fs;
use std::io;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::thread;
use std::time::Duration;

const SOCKET_PATH: &str = "/tmp/devpane.sock";

impl ReadWrite for UnixStream {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        UnixStream::set_read_timeout(self, timeout)
    }
}

pub struct Listener(UnixListener);

impl Listener {
    pub fn bind() -> io::Result<Self> {
        let listener = UnixListener::bind(SOCKET_PATH)?;
        listener.set_nonblocking(true)?;
        Ok(Self(listener))
    }

    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.0.set_nonblocking(nonblocking)
    }

    pub fn accept(&self) -> io::Result<Option<Connection>> {
        match self.0.accept() {
            Ok((stream, _)) => Ok(Some(Connection::new(stream))),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(error) => Err(error),
        }
    }
}

pub fn connect() -> io::Result<Connection> {
    UnixStream::connect(SOCKET_PATH).map(Connection::new)
}

pub fn connect_with_retry(retries: u32, delay_ms: u64) -> io::Result<Connection> {
    for attempt in 0..retries {
        match connect() {
            Ok(connection) => return Ok(connection),
            Err(error) if attempt + 1 == retries => {
                return Err(error);
            }
            Err(_) => {
                thread::sleep(Duration::from_millis(delay_ms));
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotConnected,
        "No connection attempts were made",
    ))
}

pub fn prepare_listener() -> io::Result<Listener> {
    if Path::new(SOCKET_PATH).exists() {
        match connect() {
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    "An instance is already listening",
                ));
            }
            Err(_) => {
                fs::remove_file(SOCKET_PATH)?;
            }
        }
    }

    Listener::bind()
}

pub fn remove_socket() -> io::Result<()> {
    let path = Path::new(SOCKET_PATH);

    if path.exists() {
        fs::remove_file(path)?;
    }

    Ok(())
}
