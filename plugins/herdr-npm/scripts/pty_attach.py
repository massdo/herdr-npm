#!/usr/bin/env python3
"""Keep a Herdr TUI client attached at a fixed PTY size.

Optional control FIFO (HERDR_NPM_E2E_PTY_CTL):
  click COL ROW   — SGR left-click at 1-based cells
  key HEX         — write raw bytes decoded from hex
  quit
"""

from __future__ import annotations

import fcntl
import os
import pty
import select
import struct
import sys
import termios
import time


def env(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise SystemExit(f"{name} is required")
    return value


def set_winsize(fd: int, rows: int, cols: int) -> None:
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))


def sgr_click(col: int, row: int) -> bytes:
    return f"\x1b[<0;{col};{row}M\x1b[<0;{col};{row}m".encode()


def spawn_herdr(session: str, slave: int) -> int:
    pid = os.fork()
    if pid == 0:
        os.setsid()
        fcntl.ioctl(slave, termios.TIOCSCTTY, 0)
        os.dup2(slave, 0)
        os.dup2(slave, 1)
        os.dup2(slave, 2)
        if slave > 2:
            os.close(slave)
        os.execvpe(
            "herdr",
            ["herdr", "--session", session],
            os.environ,
        )
    return pid


def main() -> int:
    session = env("HERDR_NPM_E2E_SESSION")
    rows = int(os.environ.get("HERDR_NPM_E2E_ROWS", "40"))
    cols = int(os.environ.get("HERDR_NPM_E2E_COLS", "120"))
    ctl_path = os.environ.get("HERDR_NPM_E2E_PTY_CTL")
    log_path = os.environ.get("HERDR_NPM_E2E_CLIENT_LOG")
    pid_path = os.environ.get("HERDR_NPM_E2E_CLIENT_PID")

    master, slave = pty.openpty()
    set_winsize(slave, rows, cols)
    set_winsize(master, rows, cols)
    pid = spawn_herdr(session, slave)
    if pid_path:
        with open(pid_path, "w", encoding="utf-8") as handle:
            handle.write(str(os.getpid()))
    log = open(log_path, "ab") if log_path else None
    ctl_fd = None
    if ctl_path:
        if os.path.exists(ctl_path):
            os.remove(ctl_path)
        os.mkfifo(ctl_path)
        # Reader side. O_RDWR does not keep a writer unblocked on macOS.
        ctl_fd = os.open(ctl_path, os.O_RDONLY | os.O_NONBLOCK)
        ready = os.environ.get("HERDR_NPM_E2E_PTY_READY")
        if ready:
            with open(ready, "w", encoding="utf-8") as handle:
                handle.write("1\n")

    try:
        while True:
            fds = [master]
            if ctl_fd is not None:
                fds.append(ctl_fd)
            ready_fds, _, _ = select.select(fds, [], [], 0.25)
            if master in ready_fds:
                data = os.read(master, 8192)
                if data and log:
                    log.write(data)
                    log.flush()
            if ctl_fd is not None and ctl_fd in ready_fds:
                raw = os.read(ctl_fd, 4096)
                for line in raw.decode("utf-8", "replace").splitlines():
                    parts = line.strip().split()
                    if not parts:
                        continue
                    if parts[0] == "quit":
                        return 0
                    if parts[0] == "click" and len(parts) == 3:
                        os.write(master, sgr_click(int(parts[1]), int(parts[2])))
                    elif parts[0] == "key" and len(parts) == 2:
                        os.write(master, bytes.fromhex(parts[1]))
            if os.waitpid(pid, os.WNOHANG)[0] == pid:
                time.sleep(0.2)
                pid = spawn_herdr(session, slave)
    finally:
        if log:
            log.close()
        try:
            os.kill(pid, 15)
        except OSError:
            pass
    return 0


if __name__ == "__main__":
    sys.exit(main())
