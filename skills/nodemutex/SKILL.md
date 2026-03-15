---
name: nodemutex
description: Serialize exclusive single-host work with nodemutex. Use when Codex needs to run commands that must not overlap on one machine, inspect or start a nodemutex server, choose safe Linux/macOS/FreeBSD/Windows/container endpoint overrides, or wrap jobs in `nodemutex -- ...` while keeping the same user-facing CLI across platforms.
---

# Nodemutex

Use `nodemutex` to gate commands behind a local FIFO queue. Prefer this skill when the problem is "only one of these jobs should run at a time on this machine" rather than cross-machine orchestration.

## Workflow

1. Confirm the binary you will call.
   - Prefer `nodemutex` from `PATH`.
   - In this repo, build it with `cargo build --release` and use `target/release/nodemutex` if it is not installed.
2. Choose whether to reuse the default endpoint or isolate your work.
   - Reuse the default endpoint when you intentionally want the shared queue.
   - Use a dedicated endpoint in tests, CI, temporary workspaces, or whenever you must avoid interfering with an existing server.
3. Ensure the server exists.
   - Prefer `scripts/ensure_server.py` instead of hand-managing a detached server.
   - Pass `--socket` on Unix or `--addr` on Windows when you need an isolated endpoint.
4. Run the protected command with the same environment override used for the server.
   - `nodemutex -- <command> ...`
5. Inspect queue state with `nodemutex status` when needed.

## Quick Commands

Shared/default queue:

```bash
nodemutex status
nodemutex -- make train
```

Isolated Unix queue:

```bash
python3 scripts/ensure_server.py --binary target/release/nodemutex --socket "$PWD/.nodemutex/nodemutex.sock"
NODEMUTEX_SOCK="$PWD/.nodemutex/nodemutex.sock" target/release/nodemutex -- pytest -q
```

Isolated Windows queue:

```powershell
python scripts/ensure_server.py --binary .\target\release\nodemutex.exe --addr 127.0.0.1:45231
$env:NODEMUTEX_ADDR = "127.0.0.1:45231"
.\target\release\nodemutex.exe -- cargo test
```

## Guardrails

- Keep the endpoint override identical for `server`, `status`, and protected commands. A mismatch looks like "server not running."
- Prefer a dedicated endpoint in automation. Do not point CI at a shared workstation queue unless the user explicitly wants that.
- Treat `nodemutex status` failure as connectivity or endpoint mismatch, not as lock contention.
- Preserve the public CLI. Do not invent wrapper subcommands when the user expects `nodemutex server`, `nodemutex status`, or `nodemutex -- ...`.
- On Linux, check for an existing `/run/nodemutex/nodemutex.sock` service before starting another shared server.
- Read [references/platforms.md](references/platforms.md) only when you need endpoint defaults, platform notes, or CI examples.

## Resources

- `scripts/ensure_server.py`: start or reuse a local `nodemutex server` and wait until it answers `status`
- `references/platforms.md`: endpoint defaults and examples for Linux, macOS, FreeBSD, Windows, containers, and CI
