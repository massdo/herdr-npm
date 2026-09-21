#!/usr/bin/env python3
"""One sequential journey through an isolated, attached Herdr session."""

import fcntl
import json
import os
import pty
import socket
import struct
import subprocess
import termios
import threading
import time
from pathlib import Path


def env(name):
    return os.environ[f"HERDR_NPM_E2E_{name}"]


class Client:
    def start(self):
        self.pid, self.master = pty.fork()
        if self.pid == 0:
            os.execvp("herdr", ["herdr", "--session", env("SESSION")])
        fcntl.ioctl(self.master, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
        self.reader = threading.Thread(target=self.record, daemon=True)
        self.reader.start()

    def record(self):
        with open(env("CLIENT_LOG"), "ab", buffering=0) as log:
            while True:
                try:
                    chunk = os.read(self.master, 8192)
                except OSError:
                    break  # Linux reports PTY EOF as EIO.
                if not chunk:
                    break
                log.write(chunk)

    def close(self):
        if self.pid is None:
            return
        try:
            os.kill(self.pid, 15)
        except ProcessLookupError:
            pass
        os.waitpid(self.pid, 0)
        self.reader.join(timeout=2)
        os.close(self.master)
        self.pid = None

    def shortcut(self):
        log = Path(env("CLIENT_LOG"))
        offset = log.stat().st_size
        os.write(self.master, b"\x02")
        wait(lambda: b"PREFIX" in log.read_bytes()[offset:], "client did not enter prefix mode")
        os.write(self.master, b"S")


def herdr(*args):
    result = subprocess.run(
        ["herdr", "--session", env("SESSION"), *args],
        text=True, capture_output=True, timeout=10,
    )
    assert result.returncode == 0, f"herdr {args}: {result.stdout}{result.stderr}"
    return result.stdout


def data(*args):
    return json.loads(herdr(*args))["result"]


def wait(predicate, message, timeout=15):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        value = predicate()
        if value:
            return value
        time.sleep(0.1)
    raise AssertionError(message)


def panes():
    return data("pane", "list")["panes"]


def tabs():
    return data("tab", "list")["tabs"]


def is_sidebar(pane):
    return (pane.get("tokens") or {}).get("herdr_npm_sidebar") == "v1"


def is_explorer(pane):
    return "herdr-sidebar-explorer" in (pane.get("tokens") or {}) or pane.get("label") == "Sidebar"


def sidebar():
    return next((p for p in panes() if is_sidebar(p)), None)


def read(pane):
    return herdr("pane", "read", pane, "--source", "visible", "--format", "text")


def focus(pane):
    with socket.socket(socket.AF_UNIX) as stream:
        stream.settimeout(10)
        stream.connect(env("SOCKET"))
        request = {"id": "focus", "method": "pane.focus", "params": {"pane_id": pane}}
        stream.sendall((json.dumps(request) + "\n").encode())
        response = json.loads(stream.makefile().readline())
        assert "error" not in response, response


def keys(pane, *keys):
    herdr("pane", "send-keys", pane, *keys)


def toggle():
    herdr("plugin", "action", "invoke", "herdr-npm.toggle")


def open_sidebar(working):
    focus(working)
    toggle()
    pane = wait(sidebar, "sidebar did not open")["pane_id"]
    wait(lambda: "dev" in read(pane) and "app" in read(pane), "catalogue did not render")
    return pane


def case_name():
    return os.environ.get("HERDR_NPM_E2E_CASE", "")


def write_fixture_scripts(extra=0):
    scripts = {"dev": "vite", "build": "tsc && vite build", "test": "echo TEST_OK"}
    for index in range(1, extra + 1):
        scripts[f"s{index}"] = f"echo s{index}"
    Path(env("FIXTURE"), "package.json").write_text(
        json.dumps({"name": "app", "scripts": scripts})
    )


def sgr_click(pane, col, row):
    herdr("pane", "send-text", pane, f"\x1b[<0;{col};{row}M\x1b[<0;{col};{row}m")


def prove_name_click_does_not_launch(npm):
    argv_file = Path(env("ARGV"))
    argv_file.unlink(missing_ok=True)
    before = {t["tab_id"] for t in tabs()}
    sgr_click(npm, 5, 4)
    time.sleep(0.3)
    assert not argv_file.is_file(), "name click wrote argv"
    assert {t["tab_id"] for t in tabs()} == before, "name click created a tab"
    print("name_click_does_not_launch_ok", flush=True)


def focus_pane(pane):
    focus(pane)
    wait(
        lambda: next((p for p in panes() if p["pane_id"] == pane and p.get("focused")), None),
        f"{pane} did not take focus",
    )


def sgr_wheel(pane, col, row, down=True):
    button = 65 if down else 64
    herdr("pane", "send-text", pane, f"\x1b[<{button};{col};{row}M")


def prove_wheel(npm, client):
    focus_pane(npm)
    before_tabs = {t["tab_id"] for t in tabs()}
    before = read(npm)
    sgr_wheel(npm, 2, 4, down=True)
    wait(lambda: read(npm) != before, "pane SGR wheel did not change the visible window")
    after_down = read(npm)
    assert before_tabs == {t["tab_id"] for t in tabs()}, "wheel created a tab"
    sgr_wheel(npm, 2, 4, down=False)
    wait(lambda: read(npm) != after_down, "pane SGR wheel up did not change the visible window")
    after_up = read(npm)
    time.sleep(0.5)
    assert read(npm) == after_up, "same-size redraw changed the scrolled window"
    print("pane_sgr_wheel_decode_ok", flush=True)

    layout = data("pane", "layout", "--pane", npm)["layout"]["panes"]
    rect = next(p["rect"] for p in layout if p["pane_id"] == npm)
    # Pane PTY SGR is local (row 4 = first script). The attached client
    # includes a tab bar: layout height is 39 in a 40-row terminal, so
    # screen SGR is shifted down by that chrome. Wheel on the header is a
    # no-op by contract, so the extra row is required to hit the list.
    chrome = 40 - int(rect["height"])
    col = int(rect["x"]) + 2
    row = int(rect["y"]) + 4 + chrome
    focus_pane(npm)
    before_client = read(npm)
    os.write(client.master, f"\x1b[<65;{col};{row}M".encode())
    routed = False
    deadline = time.monotonic() + 2
    while time.monotonic() < deadline:
        if read(npm) != before_client:
            routed = True
            break
        time.sleep(0.1)
    if routed:
        print("client_pty_wheel_routing_ok", flush=True)
    else:
        print(
            "client_pty_wheel_routing_missing: "
            "herdr 0.9.1 attached client did not forward a PTY SGR wheel "
            f"at col={col} row={row} to the npm pane; "
            "pane send-text remains the TUI decode proof only",
            flush=True,
        )
    assert before_tabs == {t["tab_id"] for t in tabs()}, "routed wheel created a tab"


def prove_search(npm):
    focus_pane(npm)
    before_tabs = {t["tab_id"] for t in tabs()}
    keys(npm, "/")
    keys(npm, "b")
    wait(lambda: "build" in read(npm), "search did not keep build")
    keys(npm, "Enter")
    time.sleep(0.2)
    assert before_tabs == {t["tab_id"] for t in tabs()}, "Enter in search launched a script"
    launch(npm, "build")
    print("search_filter_and_launch_ok", flush=True)


def prove_style(npm):
    layout = data("pane", "layout", "--pane", npm)["layout"]["panes"]
    rect = next(p["rect"] for p in layout if p["pane_id"] == npm)
    width = int(rect["width"])
    assert 28 <= width <= 40, f"npm pane width {width} is not near 32 columns"
    text = read(npm)
    assert "app" in text and "dev" in text, text
    assert "enter" in text.lower(), text
    if os.environ.get("HERDR_NPM_ICONS") == "ascii":
        assert ">" in text and "#" in text, text
    print("style_case_ok", flush=True)


def launch(npm, script, click=False):
    argv_file = Path(env("ARGV"))
    argv_file.unlink(missing_ok=True)
    before = {t["tab_id"] for t in tabs()}
    if click:
        # SGR column 2 is the play-icon gutter (1-based); column 5 is the name.
        # Row 4 is the first script: border, header, separator, then the list.
        sgr_click(npm, 2, 4)
    else:
        keys(npm, "j")
        # The command footer, not a row anywhere in the list, proves selection.
        def build_selected():
            lines = read(npm).splitlines()
            return any(i > 0 and "enter" in line.lower() and "vite build" in lines[i-1]
                       for i, line in enumerate(lines))
        wait(build_selected, "j did not select build")
        keys(npm, "Enter")
    wait(argv_file.is_file, f"{script} did not invoke npm")
    wait(lambda: any(t["tab_id"] not in before for t in tabs()), "no new tab")
    time.sleep(0.2)  # Process mouse release before checking that it did not relaunch.
    created = [t for t in tabs() if t["tab_id"] not in before]
    assert len(created) == 1, created
    tab = created[0]
    assert tab["label"] == f"npm run -- {script}" and not tab["focused"], tab
    assert json.loads(argv_file.read_text())[1:] == ["run", "--", script]
    # herdr-sidebar also opens an explorer in new tabs: select the ordinary PTY.
    pane = wait(lambda: next((p for p in panes() if p["tab_id"] == tab["tab_id"]
                             and not is_explorer(p) and not is_sidebar(p)), None),
                "script PTY not found")
    assert Path(pane["cwd"]).resolve() == Path(env("FIXTURE")).resolve(), pane
    return tab["tab_id"], pane["pane_id"]


def pid_alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False


def settled_layout(pane):
    previous, stable_since = None, time.monotonic()
    def settled():
        nonlocal previous, stable_since
        current = data("pane", "layout", "--pane", pane)["layout"]["panes"]
        if current != previous:
            previous, stable_since = current, time.monotonic()
        return current if time.monotonic() - stable_since >= 0.5 else None
    return wait(settled, "pane geometry did not settle")


def main(client):
    xdg = Path(env("XDG")).resolve()
    assert env("SESSION").startswith("herdr-npm-e2e-")
    assert Path(env("SOCKET")).resolve().is_relative_to(xdg)
    assert Path(env("CONFIG")).resolve().is_relative_to(xdg)
    assert os.environ["HERDR_SOCKET_PATH"] == env("SOCKET")
    assert xdg != Path.home() / ".config"

    if case_name() in ("wheel", "v1_1"):
        write_fixture_scripts(40)

    # Open the real explorer once; do not race its hooks with layout resets.
    herdr("plugin", "action", "invoke", "herdr-sidebar.show-explorer")
    explorer = wait(lambda: next((p for p in panes() if is_explorer(p)), None),
                    "explorer did not open")["pane_id"]
    working = next(p["pane_id"] for p in panes() if not is_explorer(p))
    before = settled_layout(explorer)
    explorer_rect = next(p["rect"] for p in before if p["pane_id"] == explorer)
    npm = open_sidebar(working)
    layout = settled_layout(npm)
    rects = {p["pane_id"]: p["rect"] for p in layout}
    assert len(layout) == len(before) + 1, layout
    assert rects[explorer] == explorer_rect, (before, layout)
    assert rects[npm]["x"] >= explorer_rect["x"] + explorer_rect["width"] - 1, rects
    assert rects[working]["x"] > rects[npm]["x"], rects
    print("explorer_docking_ok", flush=True)

    if case_name() == "icon":
        prove_name_click_does_not_launch(npm)
        launch(npm, "dev", click=True)
        print("icon_case_ok", flush=True)
        return
    if case_name() == "wheel":
        prove_wheel(npm, client)
        print("wheel_case_ok", flush=True)
        return
    if case_name() == "search":
        prove_search(npm)
        print("search_case_ok", flush=True)
        return
    if case_name() == "style":
        prove_style(npm)
        return
    if case_name() == "v1_1":
        prove_name_click_does_not_launch(npm)
        launch(npm, "dev", click=True)
        print("icon_case_ok", flush=True)
        prove_wheel(npm, client)
        print("wheel_case_ok", flush=True)
        prove_search(npm)
        print("search_case_ok", flush=True)
        keys(npm, "esc")
        time.sleep(0.2)
        prove_style(npm)
        print("v1_1_case_ok", flush=True)
        return
    if case_name() not in ("", "all"):
        raise AssertionError(f"unknown HERDR_NPM_E2E_CASE={case_name()!r}")

    prove_name_click_does_not_launch(npm)
    dev_tab, dev_pane = launch(npm, "dev", click=True)
    wait(lambda: "VITE_HOLD_START" in read(dev_pane), "dev output missing")
    pid = int(Path(env("HOLD") + ".pid").read_text())
    assert pid_alive(pid), "dev exited before close"
    focus(dev_pane)
    keys(dev_pane, "q")
    wait(lambda: Path(env("HOLD")).is_file() and "q" in Path(env("HOLD")).read_text(),
         "q did not reach dev")
    assert sidebar(), "q in dev closed the sidebar"
    herdr("tab", "close", dev_tab)
    wait(lambda: not pid_alive(pid), "closing dev tab left its process alive")
    assert sidebar(), "closing dev tab closed the sidebar"
    print("click_argv_cwd_and_process_lifecycle_ok", flush=True)

    focus(npm)
    build_tab, build_pane = launch(npm, "build")
    wait(lambda: "VITE_BUILD_OK" in read(build_pane), "build output missing")
    build_pid = int(Path(env("HOLD") + ".pid").read_text())
    wait(lambda: not pid_alive(build_pid), "build did not exit")
    assert any(t["tab_id"] == build_tab for t in tabs()), "build tab disappeared"
    assert "TSC_OK" in read(build_pane), "build output not retained"
    herdr("tab", "close", build_tab)
    print("keyboard_launch_and_retained_output_ok", flush=True)

    focus(npm)
    client.shortcut()
    wait(lambda: sidebar() is None, "shortcut did not close sidebar")
    wait(lambda: any(p["pane_id"] == working and p["focused"] for p in panes()),
         "closing sidebar did not return focus to the working pane")
    settled_layout(working)
    client.shortcut()
    npm = wait(sidebar, "shortcut did not reopen sidebar")["pane_id"]
    wait(lambda: "dev" in read(npm) and "app" in read(npm), "reopened sidebar not ready")
    keys(npm, "q")
    wait(lambda: all(p["pane_id"] != npm for p in panes()), "q did not close sidebar")
    print("shortcut_and_q_ok", flush=True)

    npm = open_sidebar(working)
    client.close()
    herdr("session", "stop", env("SESSION"))
    wait(lambda: not Path(env("SOCKET")).exists(), "server did not stop")
    with open(env("SERVER_LOG"), "a") as log:
        server = subprocess.Popen(["herdr", "--session", env("SESSION"), "server"],
                                  stdout=log, stderr=log)
    Path(env("SERVER_PID")).write_text(str(server.pid))
    wait(lambda: Path(env("SOCKET")).is_socket(), "server did not restart")
    client.start()
    restored = wait(lambda: next((p for p in panes() if p["pane_id"] == npm), None),
                    "sidebar pane not restored")
    assert not is_sidebar(restored), restored
    assert sidebar() is None, "plugin automatically replaced restored pane"
    herdr("pane", "close", npm)
    assert open_sidebar(working) != npm, "inert pane reused"
    print("restart_manual_recovery_ok\njourney_ok", flush=True)


if __name__ == "__main__":
    client = Client()
    client.start()
    try:
        main(client)
    except Exception:
        # Preserve useful CI evidence before the shell removes the isolated profile.
        print("== attached client ==", flush=True)
        print(Path(env("CLIENT_LOG")).read_bytes()[-6000:].decode("utf-8", "replace"))
        print("== failure panes ==", flush=True)
        try:
            print(herdr("plugin", "log", "list", "--plugin", "herdr-npm", "--limit", "5"))
            for pane in panes():
                print(pane, flush=True)
                print(read(pane["pane_id"]), flush=True)
        except Exception as error:
            print(f"diagnostics unavailable: {error}", flush=True)
        raise
    finally:
        client.close()
