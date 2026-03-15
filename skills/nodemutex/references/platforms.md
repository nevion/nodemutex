# Platform Notes

Load this file only when you need endpoint defaults or platform-specific command patterns.

## Default endpoints

- Linux: prefer `/run/nodemutex/nodemutex.sock` when that directory exists; otherwise use `$XDG_RUNTIME_DIR/nodemutex/nodemutex.sock` or `/tmp/nodemutex/nodemutex.sock`
- macOS and FreeBSD: `/tmp/nodemutex/nodemutex.sock`
- Windows: `127.0.0.1:45231`

Overrides:

- Unix: `NODEMUTEX_SOCK=/path/to/socket`
- Windows: `NODEMUTEX_ADDR=host:port`

## Recommended endpoint strategy

- Shared machine queue: reuse the default endpoint
- Tests, CI, temporary workspaces: use a dedicated endpoint under the workspace or temp directory
- Container jobs: prefer a container-local socket path unless the user explicitly wants the host queue

## Command patterns

Unix shared/default queue:

```bash
nodemutex status
nodemutex -- make test
```

Unix isolated queue:

```bash
python3 skills/nodemutex/scripts/ensure_server.py \
  --binary target/release/nodemutex \
  --socket "$PWD/.nodemutex/nodemutex.sock"

NODEMUTEX_SOCK="$PWD/.nodemutex/nodemutex.sock" \
  target/release/nodemutex -- cargo test
```

Windows isolated queue:

```powershell
python skills/nodemutex/scripts/ensure_server.py `
  --binary .\target\release\nodemutex.exe `
  --addr 127.0.0.1:45231

$env:NODEMUTEX_ADDR = "127.0.0.1:45231"
.\target\release\nodemutex.exe -- cargo test
```

## Failure interpretation

- `status` fails immediately: wrong endpoint, missing server, or missing binary
- Protected command waits after `waiting for lock...`: server is reachable and another client holds the lock
- Separate queues appear unexpectedly: the commands are not using the same endpoint override
