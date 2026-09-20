#!/bin/sh
# Isolated Herdr recipe for herdr-npm @e2e + PTY journey.
# Never stops the user default session. Never writes ~/.config/herdr/config.toml.
set -eu

PLUGIN_DIR=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
REPO_DIR=$(CDPATH= cd -- "$PLUGIN_DIR/../.." && pwd)
RUN_ID=$$
SESSION="herdr-npm-e2e-${RUN_ID}"
TMP=$(mktemp -d /tmp/hne.XXXXXX)
XDG="$TMP/xdg"
CONFIG="$XDG/herdr/config.toml"
FIXTURE="$TMP/work/app"
FS_ROOT="$TMP"
BIN="$TMP/bin"
HOLD="$TMP/hold.log"
ARGV="$TMP/argv.json"
SERVER_LOG="$TMP/herdr-server.log"
CLIENT_LOG="$TMP/herdr-client.log"
CLIENT_PID_FILE="$TMP/client.pid"
SERVER_PID_FILE="$TMP/server.pid"
PTY_CTL="$TMP/pty.ctl"
PTY_READY="$TMP/pty.ready"
ATTACH="$PLUGIN_DIR/scripts/pty_attach.py"

export HERDR_NPM_E2E=1
export HERDR_NPM_E2E_SESSION="$SESSION"
export HERDR_NPM_E2E_XDG="$XDG"
export HERDR_NPM_E2E_CONFIG="$CONFIG"
export HERDR_NPM_E2E_FS_ROOT="$FS_ROOT"
export HERDR_NPM_E2E_FIXTURE="$FIXTURE"
export HERDR_NPM_E2E_HOLD="$HOLD"
export HERDR_NPM_E2E_ARGV="$ARGV"
export HERDR_NPM_E2E_SERVER_LOG="$SERVER_LOG"
export HERDR_NPM_E2E_SERVER_PID="$SERVER_PID_FILE"
export HERDR_NPM_E2E_CLIENT_PID="$CLIENT_PID_FILE"
export HERDR_NPM_E2E_CLIENT_LOG="$CLIENT_LOG"
export HERDR_NPM_E2E_PTY_CTL="$PTY_CTL"
export HERDR_NPM_E2E_PTY_READY="$PTY_READY"
export HERDR_NPM_E2E_ATTACH="$ATTACH"
export HERDR_NPM_E2E_ROWS=40
export HERDR_NPM_E2E_COLS=120
export HERDR_PLUGIN_STATE_DIR="$TMP/state"
export XDG_CONFIG_HOME="$XDG"
export HERDR_CONFIG_PATH="$CONFIG"

cleanup() {
  status=$?
  if [ -n "${SESSION:-}" ]; then
    herdr session stop "$SESSION" >/dev/null 2>&1 || true
  fi
  if [ -f "$CLIENT_PID_FILE" ]; then
    kill "$(cat "$CLIENT_PID_FILE")" >/dev/null 2>&1 || true
  fi
  if [ -f "$SERVER_PID_FILE" ]; then
    kill "$(cat "$SERVER_PID_FILE")" >/dev/null 2>&1 || true
  fi
  rm -rf "$TMP"
  exit "$status"
}
trap cleanup EXIT INT HUP TERM

echo "== versions =="
herdr --version
rustc --version
command -v npm >/dev/null
command -v python3 >/dev/null
HERDR_VER=$(herdr --version | awk '{print $2}')
if [ "$HERDR_VER" != "0.9.1" ]; then
  echo "herdr version $HERDR_VER is not 0.9.1" >&2
  exit 1
fi
echo "npm $(npm --version)"
if command -v pnpm >/dev/null; then
  echo "pnpm $(pnpm --version)"
fi

USER_SOCK="${HOME}/.config/herdr/herdr.sock"
USER_CFG="${HOME}/.config/herdr/config.toml"
mkdir -p "$XDG/herdr" "$FIXTURE" "$BIN" "$HERDR_PLUGIN_STATE_DIR"

REAL_NPM=$(command -v npm)
REAL_PNPM=$(command -v pnpm || true)
export HERDR_NPM_E2E_REAL_NPM="$REAL_NPM"
if [ -n "$REAL_PNPM" ]; then
  export HERDR_NPM_E2E_REAL_PNPM="$REAL_PNPM"
fi

cat > "$BIN/herdr-e2e-shell" <<EOF
#!/bin/sh
export PATH="$BIN:\$PATH"
export HERDR_NPM_E2E_ARGV="$ARGV"
export HERDR_NPM_E2E_HOLD="$HOLD"
export HERDR_NPM_E2E_REAL_NPM="$REAL_NPM"
export HERDR_NPM_E2E_REAL_PNPM="${REAL_PNPM:-}"
exec /bin/sh "\$@"
EOF
chmod +x "$BIN/herdr-e2e-shell"

cat > "$CONFIG" <<EOF
onboarding = false

[experimental]
allow_nested = true

[terminal]
default_shell = "$BIN/herdr-e2e-shell"
shell_mode = "non_login"

[server]
headless_cols = 120
headless_rows = 40

[[keys.command]]
key = "cmd+shift+s"
type = "plugin_action"
command = "herdr-npm.toggle"
description = "Toggle npm scripts"

[[keys.command]]
key = "prefix+shift+s"
type = "plugin_action"
command = "herdr-npm.toggle"
description = "Toggle npm scripts"
EOF

cat > "$BIN/npm" <<PY
#!/usr/bin/env python3
import json, os, sys
path = os.environ.get("HERDR_NPM_E2E_ARGV") or r"$ARGV"
with open(path, "w", encoding="utf-8") as handle:
    json.dump(sys.argv, handle)
real = os.environ.get("HERDR_NPM_E2E_REAL_NPM") or r"$REAL_NPM"
os.execv(real, [real, *sys.argv[1:]])
PY
if [ -n "$REAL_PNPM" ]; then
  cat > "$BIN/pnpm" <<PY
#!/usr/bin/env python3
import json, os, sys
path = os.environ.get("HERDR_NPM_E2E_ARGV") or r"$ARGV"
with open(path, "w", encoding="utf-8") as handle:
    json.dump(sys.argv, handle)
real = os.environ.get("HERDR_NPM_E2E_REAL_PNPM") or r"$REAL_PNPM"
if not real:
    sys.exit(1)
os.execv(real, [real, *sys.argv[1:]])
PY
fi
cat > "$BIN/vite" <<PY
#!/usr/bin/env python3
import os, signal, sys, termios, tty
log = os.environ.get("HERDR_NPM_E2E_HOLD") or r"$HOLD"
if log:
    with open(log + ".pid", "w", encoding="utf-8") as handle:
        handle.write(str(os.getpid()))
if "build" in sys.argv:
    print("VITE_BUILD_OK")
    sys.exit(0)
print("VITE_HOLD_START", flush=True)
def stop(*_):
    sys.exit(0)
signal.signal(signal.SIGTERM, stop)
signal.signal(signal.SIGINT, stop)
try:
    tty_in = open("/dev/tty", "r", buffering=1)
    fd = tty_in.fileno()
    tty.setcbreak(fd)
except Exception:
    tty_in = sys.stdin
while True:
    ch = tty_in.read(1)
    if not ch:
        continue
    if log:
        with open(log, "a", encoding="utf-8") as handle:
            handle.write(ch)
PY
cat > "$BIN/tsc" <<'PY'
#!/usr/bin/env python3
print("TSC_OK")
PY
chmod +x "$BIN"/herdr-e2e-shell "$BIN"/npm "$BIN"/vite "$BIN"/tsc
if [ -f "$BIN/pnpm" ]; then
  chmod +x "$BIN/pnpm"
fi

export PATH="$BIN:$PATH"

echo "== build plugin =="
sh "$PLUGIN_DIR/scripts/build.sh"

echo "== start isolated server =="
herdr --session "$SESSION" server >"$SERVER_LOG" 2>&1 &
echo $! >"$SERVER_PID_FILE"
SOCKET=""
i=0
while [ "$i" -lt 50 ]; do
  CAND="$XDG/herdr/sessions/$SESSION/herdr.sock"
  if [ -S "$CAND" ]; then
    SOCKET=$CAND
    break
  fi
  i=$((i + 1))
  sleep 0.1
done
if [ -z "$SOCKET" ]; then
  echo "isolated socket did not appear" >&2
  cat "$SERVER_LOG" >&2
  exit 1
fi
export HERDR_NPM_E2E_SOCKET="$SOCKET"
export HERDR_SOCKET_PATH="$SOCKET"

if [ "$SOCKET" = "$USER_SOCK" ] || [ "$CONFIG" = "$USER_CFG" ]; then
  echo "isolation check failed: socket/config collide with the user profile" >&2
  exit 1
fi
echo "e2e_socket=$SOCKET"
echo "e2e_config=$CONFIG"
echo "user_socket=$USER_SOCK"

echo "== link plugins =="
herdr --session "$SESSION" plugin link "$PLUGIN_DIR" --enabled
SIDEBAR_ROOT=${HERDR_SIDEBAR_ROOT:-}
if [ -z "$SIDEBAR_ROOT" ]; then
  for dir in "${HOME}/.config/herdr/plugins/github"/herdr-sidebar-*; do
    if [ -f "$dir/herdr-plugin.toml" ]; then
      SIDEBAR_ROOT=$dir
      break
    fi
  done
fi
if [ -n "${SIDEBAR_ROOT:-}" ] && [ -f "$SIDEBAR_ROOT/herdr-plugin.toml" ]; then
  herdr --session "$SESSION" plugin link "$SIDEBAR_ROOT" --enabled
else
  herdr --session "$SESSION" plugin install alexarthurs/herdr-sidebar/plugins/herdr-sidebar --ref 1a5d37ef84edc91e5b3d3d4e39daa32952e6ecf2 --yes
fi
herdr --session "$SESSION" plugin list
herdr --session "$SESSION" config check

echo "== workspace =="
herdr --session "$SESSION" workspace create --cwd "$FIXTURE" --label e2e --no-focus >/dev/null

echo "== attach PTY client =="
# Attach only once the fixture workspace exists; otherwise Herdr creates a
# default workspace in the checkout and the scenario reset keeps that one.
python3 "$ATTACH" >"$TMP/attach.out" 2>"$TMP/attach.err" &
echo $! >"$CLIENT_PID_FILE"
sleep 0.4

echo "== cucumber @e2e =="
cd "$REPO_DIR"
cargo test --manifest-path "$PLUGIN_DIR/Cargo.toml" --test features -- --tags @e2e
echo "cucumber_e2e_exit=0"

echo "== PTY journey =="
i=0
while [ "$i" -lt 40 ]; do
  if [ -f "$PTY_READY" ] && [ -p "$PTY_CTL" ]; then
    break
  fi
  i=$((i + 1))
  sleep 0.1
done
if [ ! -f "$PTY_READY" ]; then
  echo "PTY attach did not become ready" >&2
  cat "$TMP/attach.err" >&2 || true
  exit 1
fi
python3 "$PLUGIN_DIR/scripts/e2e_journey.py"
echo "journey_exit=0"

echo "== isolation still holds =="
if [ -S "$USER_SOCK" ]; then
  echo "user default session socket still present (untouched)"
fi
if [ "$SOCKET" = "$USER_SOCK" ]; then
  exit 1
fi
echo "e2e_ok session=$SESSION tmp=$TMP"
