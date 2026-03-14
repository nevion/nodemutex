use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

fn socket_path() -> PathBuf {
    if let Ok(p) = std::env::var("NODEMUTEX_SOCK") {
        return PathBuf::from(p);
    }
    PathBuf::from("/run/nodemutex/nodemutex.sock")
}

// ── Server ──────────────────────────────────────────────────────────────────

struct Client {
    id: u64,
    label: String,
    stream: UnixStream,
}

struct ServerState {
    next_id: u64,
    holder: Option<u64>,
    clients: Vec<Client>,
}

impl ServerState {
    fn new() -> Self {
        Self {
            next_id: 0,
            holder: None,
            clients: Vec::new(),
        }
    }

    fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Try to grant to the next waiter if no one holds the lock.
    fn try_advance(&mut self) {
        if self.holder.is_some() {
            return;
        }
        // Find the first client that is not the holder (i.e., a waiter).
        // Clients are kept in insertion order, so first waiter = first in vec
        // that isn't the holder.
        loop {
            let waiter_pos = self
                .clients
                .iter()
                .position(|c| Some(c.id) != self.holder);
            let Some(pos) = waiter_pos else {
                return;
            };
            let client = &mut self.clients[pos];
            match client.stream.write_all(b"GRANT\n").and_then(|_| client.stream.flush()) {
                Ok(()) => {
                    let id = client.id;
                    eprintln!("nodemutex: granted to {}", client.label);
                    self.holder = Some(id);
                    return;
                }
                Err(_) => {
                    // Dead waiter, remove it.
                    eprintln!("nodemutex: {} disconnected while queued", self.clients[pos].label);
                    self.clients.remove(pos);
                }
            }
        }
    }

    fn remove_client(&mut self, id: u64) {
        let was_holder = self.holder == Some(id);
        if let Some(pos) = self.clients.iter().position(|c| c.id == id) {
            let label = self.clients[pos].label.clone();
            self.clients.remove(pos);
            if was_holder {
                eprintln!("nodemutex: {label} released");
                self.holder = None;
                self.try_advance();
            } else {
                eprintln!("nodemutex: {label} left queue");
            }
        }
    }

    fn status_string(&self) -> String {
        let mut out = String::new();
        if let Some(hid) = self.holder {
            if let Some(h) = self.clients.iter().find(|c| c.id == hid) {
                out.push_str(&format!("holder: {}\n", h.label));
            }
        } else {
            out.push_str("holder: (none)\n");
        }
        let waiters: Vec<&str> = self
            .clients
            .iter()
            .filter(|c| Some(c.id) != self.holder)
            .map(|c| c.label.as_str())
            .collect();
        if waiters.is_empty() {
            out.push_str("queue: (empty)\n");
        } else {
            out.push_str(&format!("queue ({}):\n", waiters.len()));
            for (i, w) in waiters.iter().enumerate() {
                out.push_str(&format!("  {}. {w}\n", i + 1));
            }
        }
        out
    }
}

fn run_server() -> io::Result<()> {
    let path = socket_path();

    // Clean up stale socket.
    if path.exists() {
        if UnixStream::connect(&path).is_ok() {
            eprintln!("nodemutex: server already running at {}", path.display());
            std::process::exit(1);
        }
        std::fs::remove_file(&path)?;
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let listener = UnixListener::bind(&path)?;
    eprintln!("nodemutex: server listening on {}", path.display());

    let state = Arc::new(Mutex::new(ServerState::new()));

    for incoming in listener.incoming() {
        let stream = match incoming {
            Ok(s) => s,
            Err(e) => {
                eprintln!("nodemutex: accept error: {e}");
                continue;
            }
        };

        let state = Arc::clone(&state);
        thread::spawn(move || handle_client(stream, state));
    }

    Ok(())
}

fn handle_client(stream: UnixStream, state: Arc<Mutex<ServerState>>) {
    let read_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    };
    let mut reader = BufReader::new(read_stream);

    // First line is the label.
    let mut label = String::new();
    if reader.read_line(&mut label).is_err() {
        return;
    }
    let label = label.trim().to_string();

    // Handle status requests inline (no queuing).
    if label == "__status__" {
        let s = state.lock().unwrap();
        let status = s.status_string();
        drop(s);
        let _ = (&stream).write_all(status.as_bytes());
        return;
    }

    let write_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    };

    let id;
    {
        let mut s = state.lock().unwrap();
        id = s.alloc_id();
        let mut client = Client {
            id,
            label: label.clone(),
            stream: write_stream,
        };

        if s.holder.is_none() {
            match client.stream.write_all(b"GRANT\n").and_then(|_| client.stream.flush()) {
                Ok(()) => {
                    eprintln!("nodemutex: granted to {label}");
                    s.holder = Some(id);
                    s.clients.push(client);
                }
                Err(_) => return,
            }
        } else {
            let pos = s.clients.len()
                - s.holder.map(|_| 1).unwrap_or(0);
            eprintln!("nodemutex: {label} queued (position {pos})");
            s.clients.push(client);
        }
    }

    // Block until the client disconnects.
    loop {
        let mut buf = String::new();
        match reader.read_line(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(_) => continue,
        }
    }

    // Client gone — clean up.
    let mut s = state.lock().unwrap();
    s.remove_client(id);
}

// ── Client ──────────────────────────────────────────────────────────────────

fn run_client(cmd: &[String]) -> io::Result<ExitCode> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "cannot connect to nodemutex server at {}: {e}",
                path.display()
            ),
        )
    })?;

    let label = format!("pid={} {}", std::process::id(), cmd.join(" "));
    stream.write_all(label.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut reader = BufReader::new(stream.try_clone()?);
    let mut response = String::new();
    eprintln!("nodemutex: waiting for lock...");
    reader.read_line(&mut response)?;

    if response.trim() != "GRANT" {
        eprintln!("nodemutex: unexpected response: {}", response.trim());
        return Ok(ExitCode::FAILURE);
    }

    eprintln!("nodemutex: lock acquired, running: {}", cmd.join(" "));

    let status = Command::new(&cmd[0])
        .args(&cmd[1..])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    // Drop stream → server detects disconnect → releases lock.
    drop(reader);
    drop(stream);

    let code = status.code().unwrap_or(128);
    Ok(ExitCode::from(code as u8))
}

// ── Status ──────────────────────────────────────────────────────────────────

fn run_status() -> io::Result<()> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path).map_err(|e| {
        io::Error::new(e.kind(), format!("cannot connect to server: {e}"))
    })?;

    stream.write_all(b"__status__\n")?;
    stream.flush()?;

    let reader = BufReader::new(stream);
    for line in reader.lines() {
        println!("{}", line?);
    }
    Ok(())
}

// ── Main ────────────────────────────────────────────────────────────────────

fn usage() -> ! {
    eprintln!(
        "\
Usage:
  nodemutex server              Start the lock server
  nodemutex [--] CMD [ARGS...]  Wait for exclusive access, then run CMD
  nodemutex status              Show queue status"
    );
    std::process::exit(2);
}

/// Extract the command to run from args. Supports:
///   nodemutex -- sleep 10
///   nodemutex sleep 10
fn extract_cmd(args: &[String]) -> &[String] {
    // Skip past "--" if present, otherwise take everything after argv[0].
    if let Some(pos) = args.iter().position(|a| a == "--") {
        &args[pos + 1..]
    } else {
        &args[1..]
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
    }

    match args[1].as_str() {
        "server" => {
            if let Err(e) = run_server() {
                eprintln!("nodemutex server error: {e}");
                return ExitCode::FAILURE;
            }
        }
        "status" => {
            if let Err(e) = run_status() {
                eprintln!("nodemutex: {e}");
                return ExitCode::FAILURE;
            }
        }
        _ => {
            let cmd = extract_cmd(&args);
            if cmd.is_empty() {
                usage();
            }
            match run_client(&cmd.to_vec()) {
                Ok(code) => return code,
                Err(e) => {
                    eprintln!("nodemutex: {e}");
                    return ExitCode::FAILURE;
                }
            }
        }
    }

    ExitCode::SUCCESS
}
