use super::{Connection, ReadWrite};

use interprocess::local_socket::{
    GenericNamespaced, Listener as LocalListener, ListenerNonblockingMode, ListenerOptions,
    Stream as LocalStream, ToNsName,
    traits::{Listener as _, Stream as _},
};
use std::io;
use std::thread;
use std::time::Duration;

const SOCKET_NAME: &str = "devpane";

fn socket_name() -> io::Result<interprocess::local_socket::Name<'static>> {
    SOCKET_NAME.to_ns_name::<GenericNamespaced>()
}

impl ReadWrite for LocalStream {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.set_recv_timeout(timeout)
    }
}

pub struct Listener(LocalListener);

impl Listener {
    pub fn bind() -> io::Result<Self> {
        let name = socket_name()?;

        let listener = ListenerOptions::new()
            .name(name)
            .nonblocking(ListenerNonblockingMode::Accept)
            .create_sync()
            .map_err(io::Error::other)?;

        Ok(Self(listener))
    }

    pub fn accept(&self) -> io::Result<Option<Connection>> {
        match self.0.accept() {
            Ok(stream) => Ok(Some(Connection::new(stream))),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(error) => Err(error),
        }
    }
}

pub fn connect() -> io::Result<Connection> {
    let name = socket_name()?;

    LocalStream::connect(name)
        .map(Connection::new)
        .map_err(io::Error::other)
}

pub fn connect_with_retry(retries: u32, delay_ms: u64) -> io::Result<Connection> {
    for attempt in 0..retries {
        match connect() {
            Ok(connection) => return Ok(connection),
            Err(error) if attempt + 1 == retries => return Err(error),
            Err(_) => thread::sleep(Duration::from_millis(delay_ms)),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotConnected,
        "No connection attempts were made",
    ))
}

pub fn prepare_listener() -> io::Result<Listener> {
    Listener::bind()
}

pub fn remove_socket() -> io::Result<()> {
    // Windows named pipes are not filesystem socket files.
    // Closing the listener releases the pipe endpoint.
    Ok(())
}
