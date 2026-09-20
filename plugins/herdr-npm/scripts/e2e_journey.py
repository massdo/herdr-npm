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


def click_first_script(pane_id: str) -> None:
    # A single SGR down/up pair reaches the real plugin PTY at its first row.
    result = herdr("pane", "send-text", pane_id, "\x1b[<0;5;3M\x1b[<0;5;3m")
    if result.returncode != 0:
        raise SystemExit(f"mouse input failed: {result.stdout}{result.stderr}")


def toggle_shortcut() -> None:
    ctl("key 02")  # ctrl+b (the Herdr prefix)
    time.sleep(0.1)
    ctl("key 1b5b3131353b3275")  # CSI 115;2u: Shift+S, including its modifier


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

    before_tabs = {tab["tab_id"] for tab in herdr_json("tab", "list")["result"]["tabs"]}
    # Click once in the plugin PTY; pre-existing script tabs cannot satisfy this check.
    click_first_script(npm["pane_id"])
    wait(argv_file.is_file, message="click did not invoke the package manager")
    wait(lambda: len(herdr_json("tab", "list")["result"]["tabs"]) > len(before_tabs),
         message="click did not create a new tab")
    time.sleep(0.2)  # Let the release event be processed before checking no second launch.
    tabs = herdr_json("tab", "list")["result"]["tabs"]
    created = [tab for tab in tabs if tab["tab_id"] not in before_tabs]
    if len(created) != 1:
        raise SystemExit(f"one click must create exactly one tab: {created}")
    if created[0].get("label") != "npm run -- dev":
        raise SystemExit(f"click selected the wrong script: {created}")
    if created[0].get("focused") is True:
        raise SystemExit("script tab stole the focus")
    argv = json.loads(argv_file.read_text())
    if argv[1:] != ["run", "--", "dev"]:
        raise SystemExit(f"unexpected argv {argv}")

    script_pane = [
        pane
        for pane in panes()
        if pane.get("tab_id") == created[-1]["tab_id"]
    ][0]
    cwd = script_pane.get("cwd") or ""
    if Path(cwd).resolve() != fixture.resolve():
        raise SystemExit(f"cwd {cwd} is not the fixture {fixture}")

    # The portable shortcut must toggle both ways through the attached Herdr client.
    herdr_json("plugin", "pane", "focus", npm["pane_id"])
    toggle_shortcut()
    wait(lambda: sidebar() is None, message="prefix+shift+s did not close the sidebar")
    toggle_shortcut()
    wait(lambda: sidebar() is not None, message="prefix+shift+s did not reopen the sidebar")

    print("journey_ok")
    print(f"tabs={len(herdr_json('tab', 'list')['result']['tabs'])}")
    print(f"cwd={cwd}")
    print("shortcut_toggle_ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
