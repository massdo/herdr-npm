#!/bin/sh
# Fresh-profile install from a published git SHA.
# Usage: sh plugins/herdr-npm/scripts/install-smoke.sh <SHA>
set -eu

SHA=${1:-}
if [ -z "$SHA" ]; then
  echo "usage: $0 <git-sha>" >&2
  exit 2
fi

PLUGIN_DIR=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
RUN_ID=$$
SESSION="herdr-npm-ins-${RUN_ID}"
TMP=$(mktemp -d /tmp/hni.XXXXXX)
XDG="$TMP/xdg"
CONFIG="$XDG/herdr/config.toml"
FIXTURE="$TMP/app"

export XDG_CONFIG_HOME="$XDG"
export HERDR_CONFIG_PATH="$CONFIG"
export XDG_STATE_HOME="$TMP/state"
export HERDR_PLUGIN_STATE_DIR="$XDG_STATE_HOME/herdr/plugins/herdr-npm"

cleanup() {
  status=$?
  herdr session stop "$SESSION" >/dev/null 2>&1 || true
  rm -rf "$TMP"
  exit "$status"
}
trap cleanup EXIT INT HUP TERM

mkdir -p "$XDG/herdr" "$FIXTURE" "$HERDR_PLUGIN_STATE_DIR"
cat > "$CONFIG" <<'EOF'
onboarding = false
[terminal]
default_shell = "/bin/sh"
shell_mode = "non_login"
[server]
headless_cols = 120
headless_rows = 40
EOF
cat > "$FIXTURE/package.json" <<'EOF'
{
  "name": "install-smoke",
  "scripts": {
    "hello": "echo INSTALL_SMOKE_OK"
  }
}
EOF

echo "== herdr $(herdr --version) rustc $(rustc --version) =="
if [ "$(herdr --version | awk '{print $2}')" != "0.9.1" ]; then
  echo "Herdr 0.9.1 is required" >&2
  exit 1
fi

USER_SOCK="${HOME}/.config/herdr/herdr.sock"
herdr --session "$SESSION" server >"$TMP/server.log" 2>&1 &
i=0
SOCKET=""
while [ "$i" -lt 50 ]; do
  CAND="$XDG/herdr/sessions/$SESSION/herdr.sock"
  if [ -S "$CAND" ]; then
    SOCKET=$CAND
    break
  fi
  i=$((i + 1))
  sleep 0.1
done
[ -n "$SOCKET" ]
export HERDR_SOCKET_PATH="$SOCKET"
if [ "$SOCKET" = "$USER_SOCK" ]; then
  echo "install-smoke socket collided with the user session" >&2
  exit 1
fi

echo "== plugin install massdo/herdr-npm/plugins/herdr-npm --ref $SHA =="
herdr --session "$SESSION" plugin install massdo/herdr-npm/plugins/herdr-npm --ref "$SHA" --yes
herdr --session "$SESSION" plugin list
herdr --session "$SESSION" config check

herdr --session "$SESSION" workspace create --cwd "$FIXTURE" --label install-smoke --no-focus >/dev/null
herdr --session "$SESSION" plugin action invoke herdr-npm.toggle
sleep 1
LIST=$(herdr --session "$SESSION" pane list)
echo "$LIST" | grep herdr_npm_sidebar >/dev/null
PANE=$(python3 -c "import json,sys; panes=json.loads(sys.stdin.read())['result']['panes'];
print(next(p['pane_id'] for p in panes if (p.get('tokens') or {}).get('herdr_npm_sidebar')=='v1'))" <<EOF
$LIST
EOF
)
TEXT=$(herdr --session "$SESSION" pane read "$PANE" --source visible --format text)
echo "$TEXT" | grep hello >/dev/null
herdr --session "$SESSION" pane send-keys "$PANE" Enter
sleep 2
TABS=$(herdr --session "$SESSION" tab list)
echo "$TABS" | grep "npm run -- hello" >/dev/null
herdr --session "$SESSION" pane send-keys "$PANE" q
sleep 0.5
AFTER=$(herdr --session "$SESSION" pane list)
echo "$AFTER" | grep herdr_npm_sidebar >/dev/null && {
  echo "sidebar still present after q" >&2
  exit 1
}
echo "install_smoke_ok ref=$SHA session=$SESSION"
