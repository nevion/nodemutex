#!/usr/bin/env python3

import argparse
import os
import subprocess
import sys
import time
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Start or reuse a nodemutex server and wait until it is ready."
    )
    parser.add_argument("--binary", default="nodemutex", help="Path to the nodemutex binary")
    parser.add_argument("--socket", help="Unix socket path override")
    parser.add_argument("--addr", help="Windows TCP address override, for example 127.0.0.1:45231")
    parser.add_argument("--log-file", help="Log file for the started server")
    parser.add_argument("--timeout", type=float, default=5.0, help="Seconds to wait for readiness")
    return parser.parse_args()


def build_env(args: argparse.Namespace) -> tuple[dict[str, str], str | None, str | None]:
    if args.socket and args.addr:
        raise SystemExit("pass only one of --socket or --addr")

    env = os.environ.copy()
    if args.socket:
        env["NODEMUTEX_SOCK"] = args.socket
        return env, "NODEMUTEX_SOCK", args.socket
    if args.addr:
        env["NODEMUTEX_ADDR"] = args.addr
        return env, "NODEMUTEX_ADDR", args.addr
    return env, None, None


def status_ready(binary: str, env: dict[str, str]) -> bool:
    try:
        result = subprocess.run(
            [binary, "status"],
            env=env,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
    except FileNotFoundError as exc:
        raise SystemExit(f"nodemutex binary not found: {binary}") from exc
    return result.returncode == 0


def default_log_file() -> Path:
    return Path.cwd() / ".nodemutex" / "server.log"


def spawn_server(binary: str, env: dict[str, str], log_file: Path) -> subprocess.Popen[bytes]:
    log_file.parent.mkdir(parents=True, exist_ok=True)
    log_handle = log_file.open("ab")

    kwargs: dict[str, object] = {
        "stdin": subprocess.DEVNULL,
        "stdout": log_handle,
        "stderr": log_handle,
        "env": env,
        "close_fds": True,
    }
    if os.name == "nt":
        kwargs["creationflags"] = (
            subprocess.DETACHED_PROCESS | subprocess.CREATE_NEW_PROCESS_GROUP
        )
    else:
        kwargs["start_new_session"] = True

    return subprocess.Popen([binary, "server"], **kwargs)


def wait_until_ready(binary: str, env: dict[str, str], timeout: float) -> bool:
    deadline = time.time() + timeout
    while time.time() < deadline:
        if status_ready(binary, env):
            return True
        time.sleep(0.1)
    return False


def main() -> int:
    args = parse_args()
    env, env_name, env_value = build_env(args)

    if status_ready(args.binary, env):
        print("status=running")
        print("started=false")
        if env_name and env_value:
            print(f"env_name={env_name}")
            print(f"env_value={env_value}")
        return 0

    log_file = Path(args.log_file) if args.log_file else default_log_file()
    proc = spawn_server(args.binary, env, log_file)

    if not wait_until_ready(args.binary, env, args.timeout):
        print(f"failed to start nodemutex server; see {log_file}", file=sys.stderr)
        return 1

    print("status=running")
    print("started=true")
    print(f"pid={proc.pid}")
    print(f"log_file={log_file}")
    if env_name and env_value:
        print(f"env_name={env_name}")
        print(f"env_value={env_value}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
