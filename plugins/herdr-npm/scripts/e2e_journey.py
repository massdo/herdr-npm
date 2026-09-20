#!/usr/bin/env python3
"""PTY-driven journey: toggle, list, click, launch, observe argv/cwd, close."""

import json
import os
import subprocess
import sys
import time
from pathlib import Path


def env(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise SystemExit(f"{name} is required")
    return value


def herdr(*args: str) -> subprocess.CompletedProcess[str]:
    cmd = ["herdr", "--session", env("HERDR_NPM_E2E_SESSION"), *args]
    try:
        return subprocess.run(
            cmd,
            check=False,
            text=True,
            capture_output=True,
            timeout=10,
            env=os.environ,
        )
    except subprocess.TimeoutExpired as error:
        raise SystemExit(f"herdr {args} timed out: {error}") from error


def herdr_json(*args: str) -> dict:
    proc = herdr(*args)
    if proc.returncode != 0:
        raise SystemExit(f"herdr {args} failed: {proc.stdout}{proc.stderr}")
    return json.loads(proc.stdout)


def ctl(line: str) -> None:
    path = env("HERDR_NPM_E2E_PTY_CTL")
    payload = (line + "\n").encode()
    deadline = time.time() + 5
    last_error: OSError | None = None
    while time.time() < deadline:
        try:
            fd = os.open(path, os.O_WRONLY | os.O_NONBLOCK)
            try:
                os.write(fd, payload)
                return
            finally:
                os.close(fd)
        except OSError as error:
            last_error = error
            time.sleep(0.05)
    raise SystemExit(f"pty ctl {line!r} failed: {last_error}")


def wait(predicate, timeout: float = 8.0, message: str = "journey timed out") -> None:
    start = time.time()
    while time.time() - start < timeout:
        if predicate():
            return
        time.sleep(0.15)
    raise SystemExit(message)


def panes() -> list[dict]:
    return herdr_json("pane", "list")["result"]["panes"]


def sidebar() -> dict | None:
    for pane in panes():
        tokens = pane.get("tokens") or {}
        if tokens.get("herdr_npm_sidebar") == "v1":
            return pane
    return None


def sidebar_rect(pane_id: str) -> dict:
    layout = herdr_json("pane", "layout", "--pane", pane_id)["result"]["layout"]
    for pane in layout.get("panes") or []:
        if pane.get("pane_id") == pane_id:
            return pane.get("rect") or {}
    return {}


def click_first_script(pane_id: str) -> None:
    rect = sidebar_rect(pane_id)
    print(f"sidebar_rect={rect}", flush=True)
    x = int(rect.get("x") or 0)
    y = int(rect.get("y") or 0)
    # SGR is 1-based. The first script sits under the pane border + header.
    spots = [(x + dx, y + dy) for dy in (2, 3, 4, 5, 6) for dx in (3, 4, 8)]
    spots.extend([(8, 3), (5, 4), (4, 3)])
    for col, row in spots:
        ctl(f"click {max(1, col)} {max(1, row)}")
    # Pane-local mouse into the plugin PTY (real terminal, not the BDD TestBackend).
    herdr("pane", "send-text", pane_id, "\x1b[<0;5;3M\x1b[<0;5;3m")


def main() -> int:
    print("journey_start", flush=True)
    fixture = Path(env("HERDR_NPM_E2E_FIXTURE"))
    argv_file = Path(env("HERDR_NPM_E2E_ARGV"))
    if argv_file.exists():
        argv_file.unlink()

    if sidebar() is None:
        herdr_json("plugin", "action", "invoke", "herdr-npm.toggle")
        wait(lambda: sidebar() is not None)

    npm = sidebar()
    assert npm is not None
    text = herdr("pane", "read", npm["pane_id"], "--source", "visible", "--format", "text").stdout
    if "dev" not in text or "build" not in text:
        raise SystemExit(f"catalogue missing scripts:\n{text}")

    # Click the first script row through the attached client PTY.
    click_first_script(npm["pane_id"])
    wait(
        lambda: argv_file.is_file() or len(herdr_json("tab", "list")["result"]["tabs"]) >= 2,
        message=f"click did not launch a script; tabs={herdr('tab', 'list').stdout}",
    )

    tabs = herdr_json("tab", "list")["result"]["tabs"]
    if len(tabs) < 2:
        raise SystemExit(f"click did not open a tab: {tabs}")
    created = [tab for tab in tabs if tab.get("label", "").startswith("npm run --")]
    if not created:
        raise SystemExit(f"no labelled script tab: {tabs}")
    if created[-1].get("focused") is True:
        raise SystemExit("script tab stole the focus")

    if argv_file.is_file():
        argv = json.loads(argv_file.read_text())
        if argv[1:4] not in (["run", "--", "dev"], ["run", "--", "hello"]):
            # first visible script of the fixture is hello or dev
            if not (len(argv) == 4 and argv[1] == "run" and argv[2] == "--"):
                raise SystemExit(f"unexpected argv {argv}")

    script_pane = [
        pane
        for pane in panes()
        if pane.get("tab_id") == created[-1]["tab_id"]
    ][0]
    cwd = script_pane.get("cwd") or ""
    if str(fixture) not in cwd and fixture.name not in cwd:
        raise SystemExit(f"cwd {cwd} is not the fixture {fixture}")

    # Shortcut: prefix+shift+s on Linux / when Ghostty is absent.
    # ctrl+b then S
    before = 1 if sidebar() else 0
    ctl("key 0253")  # STX + 'S'
    time.sleep(0.8)
    # The chord may toggle closed; reopen with the CLI if needed so cleanup is stable.
    if sidebar() is None:
        herdr_json("plugin", "action", "invoke", "herdr-npm.toggle")
        wait(lambda: sidebar() is not None)

    print("journey_ok")
    print(f"tabs={len(herdr_json('tab', 'list')['result']['tabs'])}")
    print(f"cwd={cwd}")
    print(f"shortcut_before_sidebar={before}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
