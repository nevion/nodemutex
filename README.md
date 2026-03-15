# nodemutex

A dead-simple exclusive access queue for a single machine. Think slurm, but for one node.

A server holds a FIFO queue. Clients connect, wait their turn, get exclusive access, run their command, and release. If a job crashes (even `SIGKILL`), the server detects the socket disconnect and immediately grants to the next waiter.

## Usage

```bash
# Run a command with exclusive access (waits if someone else has the lock):
nodemutex ./train.py --epochs 100

# Or with explicit separator:
nodemutex -- ./benchmark --size 4096

# See who's running and who's queued:
nodemutex status
```

## Install

```bash
./install.sh
```

On Linux with `systemd`, this builds the release binary, installs it to `/usr/local/bin/nodemutex`, and enables a system-wide service.

For other platforms, build normally and start the server yourself:

```bash
cargo build --release
./target/release/nodemutex server
```

The CLI stays the same on every platform:

- Linux, macOS, FreeBSD, and Linux containers use Unix domain sockets.
- Windows uses a loopback TCP listener internally.
- Users still run the same commands: `nodemutex server`, `nodemutex status`, and `nodemutex -- CMD ...`.

## Platform Defaults

- Linux prefers `/run/nodemutex/nodemutex.sock` when that service directory exists.
- Linux without `/run/nodemutex` falls back to `$XDG_RUNTIME_DIR/nodemutex/nodemutex.sock` when available, otherwise `/tmp/nodemutex/nodemutex.sock`.
- macOS and FreeBSD use `/tmp/nodemutex/nodemutex.sock`.
- Windows uses `127.0.0.1:45231`.

Overrides:

- Unix: set `NODEMUTEX_SOCK`
- Windows: set `NODEMUTEX_ADDR`

## Binaries

GitHub Actions builds release archives for Linux, macOS, and Windows on tagged releases. Download the matching asset from the repository's Releases page.

## How it works

- **Server** listens on a local transport and maintains a FIFO queue of clients.
- **Client** connects, sends a label, and blocks until the server writes `GRANT`.
- When the client's command exits (or crashes), the socket closes, the server detects EOF, and grants to the next waiter.

No external dependencies — pure Rust standard library.
