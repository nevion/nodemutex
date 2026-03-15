use std::io;

#[cfg(unix)]
use std::path::PathBuf;

#[cfg(windows)]
pub use std::net::{TcpListener as Listener, TcpStream as Stream};
#[cfg(unix)]
pub use std::os::unix::net::{UnixListener as Listener, UnixStream as Stream};

#[cfg(unix)]
pub fn bind() -> io::Result<Listener> {
    let path = socket_path();

    if path.exists() {
        if Stream::connect(&path).is_ok() {
            return Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                format!("server already running at {}", path.display()),
            ));
        }
        std::fs::remove_file(&path)?;
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    Listener::bind(&path)
}

#[cfg(windows)]
pub fn bind() -> io::Result<Listener> {
    let addr = socket_addr();
    match Listener::bind(addr.as_str()) {
        Ok(listener) => Ok(listener),
        Err(err) if err.kind() == io::ErrorKind::AddrInUse => {
            if Stream::connect(addr.as_str()).is_ok() {
                return Err(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    format!("server already running at {addr}"),
                ));
            }
            Err(err)
        }
        Err(err) => Err(err),
    }
}

#[cfg(unix)]
pub fn connect() -> io::Result<Stream> {
    Stream::connect(socket_path())
}

#[cfg(windows)]
pub fn connect() -> io::Result<Stream> {
    Stream::connect(socket_addr())
}

#[cfg(unix)]
pub fn endpoint_display() -> String {
    socket_path().display().to_string()
}

#[cfg(windows)]
pub fn endpoint_display() -> String {
    socket_addr()
}

#[cfg(unix)]
fn socket_path() -> PathBuf {
    if let Ok(path) = std::env::var("NODEMUTEX_SOCK") {
        return PathBuf::from(path);
    }
    default_socket_path()
}

#[cfg(target_os = "linux")]
fn default_socket_path() -> PathBuf {
    let system_path = PathBuf::from("/run/nodemutex/nodemutex.sock");
    if system_path.parent().is_some_and(|parent| parent.exists()) {
        return system_path;
    }

    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime_dir)
            .join("nodemutex")
            .join("nodemutex.sock");
    }

    PathBuf::from("/tmp/nodemutex/nodemutex.sock")
}

#[cfg(all(unix, not(target_os = "linux")))]
fn default_socket_path() -> PathBuf {
    PathBuf::from("/tmp/nodemutex/nodemutex.sock")
}

#[cfg(windows)]
fn socket_addr() -> String {
    std::env::var("NODEMUTEX_ADDR").unwrap_or_else(|_| "127.0.0.1:45231".to_string())
}
