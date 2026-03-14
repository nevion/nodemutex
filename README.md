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

This builds the release binary, installs it to `/usr/local/bin/nodemutex`, and enables a system-wide systemd service. All users share one queue via `/run/nodemutex/nodemutex.sock`.

To override the socket path, set `NODEMUTEX_SOCK`.

## How it works

- **Server** listens on a Unix domain socket and maintains a FIFO queue of clients.
- **Client** connects, sends a label, and blocks until the server writes `GRANT`.
- When the client's command exits (or crashes), the socket closes, the server detects EOF, and grants to the next waiter.

No external dependencies — pure Rust standard library.
