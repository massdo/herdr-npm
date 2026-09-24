#!/bin/sh
# Isolated Herdr recipe for herdr-npm PTY journey.
# Never stops the user default session. Never writes ~/.config/herdr/config.toml.
set -eu

PLUGIN_DIR=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
RUN_ID=$$
SESSION="herdr-npm-e2e-${RUN_ID}"
TMP=$(mktemp -d /tmp/hne.XXXXXX)
XDG="$TMP/xdg"
CONFIG="$XDG/herdr/config.toml"
FIXTURE="$TMP/work/app"
BIN="$TMP/bin"
HOLD="$TMP/hold.log"
ARGV="$TMP/argv.json"
MANAGER_LOG="$TMP/manager-launches.jsonl"
SERVER_LOG="$TMP/herdr-server.log"
CLIENT_LOG="$TMP/herdr-client.log"
SERVER_PID_FILE="$TMP/server.pid"

export HERDR_NPM_E2E_SESSION="$SESSION"
export HERDR_NPM_E2E_XDG="$XDG"
export HERDR_NPM_E2E_CONFIG="$CONFIG"
export HERDR_NPM_E2E_FIXTURE="$FIXTURE"
export HERDR_NPM_E2E_HOLD="$HOLD"
export HERDR_NPM_E2E_ARGV="$ARGV"
export HERDR_NPM_E2E_MANAGER_LOG="$MANAGER_LOG"
export HERDR_NPM_E2E_SERVER_LOG="$SERVER_LOG"
export HERDR_NPM_E2E_SERVER_PID="$SERVER_PID_FILE"
export HERDR_NPM_E2E_CLIENT_LOG="$CLIENT_LOG"
export XDG_STATE_HOME="$TMP/state"
export HERDR_PLUGIN_STATE_DIR="$XDG_STATE_HOME/herdr/plugins/herdr-npm"
export XDG_CONFIG_HOME="$XDG"
export HERDR_CONFIG_PATH="$CONFIG"

cleanup() {
  status=$?
  if [ -n "${SESSION:-}" ]; then
    herdr session stop "$SESSION" >/dev/null 2>&1 || true
  fi
  if [ -f "$SERVER_PID_FILE" ]; then
    kill "$(cat "$SERVER_PID_FILE")" >/dev/null 2>&1 || true
  fi
  if [ -n "${HERDR_NPM_E2E_ARTIFACTS:-}" ]; then
    mkdir -p "$HERDR_NPM_E2E_ARTIFACTS"
    cp -R "$TMP/." "$HERDR_NPM_E2E_ARTIFACTS/"
    echo "e2e_artifacts=$HERDR_NPM_E2E_ARTIFACTS"
  fi
  rm -rf "$TMP"
  exit "$status"
}
trap cleanup EXIT INT HUP TERM

echo "== versions =="
herdr --version
rustc --version
command -v npm >/dev/null
command -v pnpm >/dev/null
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

# Keep the real explorer installed, with deterministic preferences and no
# automatic creation in script tabs. Its actions use Herdr's per-plugin state.
mkdir -p "$XDG_STATE_HOME/herdr/plugins/herdr-sidebar"
cat > "$XDG_STATE_HOME/herdr/plugins/herdr-sidebar/state.json" <<'JSON'
{"auto_open":false,"font_prompt":true,"sidebar_width":32}
JSON

REAL_NPM=$(command -v npm)
REAL_PNPM=$(command -v pnpm)
cat > "$BIN/herdr-e2e-shell" <<EOF
#!/bin/sh
export PATH="$BIN:\$PATH"
export HERDR_NPM_E2E_ARGV="$ARGV"
export HERDR_NPM_E2E_HOLD="$HOLD"
# The monorepo fixture pins pnpm@10.10.0. Another installed pnpm would reinstall
# that version through PATH, and the recorder would log a second pnpm call.
export npm_config_manage_package_manager_versions=false
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
manager = os.path.basename(sys.argv[0])
with open(r"$MANAGER_LOG", "a", encoding="utf-8") as handle:
    handle.write(json.dumps({"manager": manager, "argv": sys.argv[1:], "cwd": os.getcwd()}) + "\n")
real = (os.environ.get("HERDR_NPM_E2E_REAL_NPM") or r"$REAL_NPM") if manager == "npm" else r"$REAL_PNPM"
os.execv(real, [real, *sys.argv[1:]])
PY
cp "$BIN/npm" "$BIN/pnpm"
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
chmod +x "$BIN"/herdr-e2e-shell "$BIN"/npm "$BIN"/pnpm "$BIN"/vite "$BIN"/tsc
export PATH="$BIN:$PATH"

echo "== build plugin =="
sh "$PLUGIN_DIR/scripts/build.sh"

if { [ "${HERDR_NPM_E2E_CASE:-}" = "style" ] || [ "${HERDR_NPM_E2E_CASE:-}" = "v1_1" ]; } && [ -z "${HERDR_NPM_ICONS:-}" ]; then
  export HERDR_NPM_ICONS=ascii
fi

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

cat > "$FIXTURE/package.json" <<'JSON'
{"name":"app","scripts":{"dev":"vite","build":"tsc && vite build","test":"echo TEST_OK"}}
JSON

echo "== workspace =="
herdr --session "$SESSION" workspace create --cwd "$FIXTURE" --label e2e --no-focus >/dev/null

echo "== PTY journey =="
# HERDR_NPM_E2E_CASE=icon limits the attached journey to the icon/name click proof.
# HERDR_NPM_E2E_CASE=wheel limits it to molette + same-size redraw (needs a long catalogue).
# HERDR_NPM_E2E_CASE=search limits it to opening search, typing, and launching a hit.
# HERDR_NPM_E2E_CASE=style checks the 32-column catalogue chrome (ASCII glyphs when forced).
# HERDR_NPM_E2E_CASE=v1_1 runs the short V1.1 path: icon/name, wheel, search+launch, style.
# HERDR_NPM_E2E_CASE=monorepo checks package groups and launches from root/member.
if { [ "${HERDR_NPM_E2E_CASE:-}" = "style" ] || [ "${HERDR_NPM_E2E_CASE:-}" = "v1_1" ]; } && [ -z "${HERDR_NPM_ICONS:-}" ]; then
  export HERDR_NPM_ICONS=ascii
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
