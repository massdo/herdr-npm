#!/bin/sh
# Fresh-profile install from a published git SHA.
# Usage: sh scripts/install-smoke.sh <SHA> [--prebuilt]
# --prebuilt validates a release install: pass the full SHA of a published
# tag. The run happens in a disposable environment without Rust and checks
# that the verified release binary was installed, not compiled.
set -eu

SHA=${1:-}
PREBUILT=0
if [ "${2:-}" = --prebuilt ]; then
  PREBUILT=1
elif [ -z "$SHA" ] || [ -n "${2:-}" ]; then
  echo "usage: $0 <git-sha> [--prebuilt]" >&2
  exit 2
fi

PLUGIN_DIR=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
RUN_ID=$$
SESSION="herdr-npm-ins-${RUN_ID}"
TMP=$(mktemp -d /tmp/hni.XXXXXX)
DIAG=$(mktemp -d /tmp/hni-diag.XXXXXX)
XDG="$TMP/xdg"
CONFIG="$XDG/herdr/config.toml"
FIXTURE="$TMP/app"
USER_SOCK="${HOME}/.config/herdr/herdr.sock"

# Never reach the Herdr session this script may be started from.
unset HERDR_SOCKET_PATH HERDR_BIN_PATH HERDR_ENV HERDR_PANE_ID HERDR_TAB_ID HERDR_WORKSPACE_ID
export XDG_CONFIG_HOME="$XDG"
export HERDR_CONFIG_PATH="$CONFIG"
export XDG_STATE_HOME="$TMP/state"
export HERDR_PLUGIN_STATE_DIR="$XDG_STATE_HOME/herdr/plugins/herdr-npm"
# Herdr 0.9.1 hides the output of a successful build.
export HERDR_NPM_BUILD_LOG="$DIAG/build.log"

cleanup() {
  status=$?
  herdr session stop "$SESSION" >/dev/null 2>&1 || true
  rm -rf "$TMP"
  if [ "$status" -eq 0 ]; then
    rm -rf "$DIAG"
  else
    echo "install-smoke failed; diagnostics kept in $DIAG" >&2
  fi
  exit "$status"
}
trap cleanup EXIT INT HUP TERM

if [ "$PREBUILT" = 1 ]; then
  # Disposable environment: an empty HOME, and a PATH holding Herdr, Git,
  # Node, npm and Python next to the system directories only.
  mkdir -p "$TMP/home" "$TMP/tools"
  for tool in herdr git node npm python3; do
    path=$(command -v "$tool") || {
      echo "$tool is required" >&2
      exit 1
    }
    ln -s "$path" "$TMP/tools/$tool"
  done
  export HOME="$TMP/home"
  export PATH="$TMP/tools:/usr/bin:/bin:/usr/sbin:/sbin"
  unset CARGO_HOME RUSTUP_HOME RUSTUP_TOOLCHAIN
  if command -v cargo >/dev/null 2>&1 || command -v rustc >/dev/null 2>&1 || [ -e "$HOME/.cargo/env" ]; then
    echo "--prebuilt needs a machine without Rust in /usr/bin, /bin, /usr/sbin and /sbin" >&2
    exit 1
  fi
  echo "== no Rust: cargo and rustc unreachable, no $HOME/.cargo/env =="
fi

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
# The witness file proves that the script ran; a tab label does not.
cat > "$FIXTURE/package.json" <<'EOF'
{
  "name": "install-smoke",
  "scripts": {
    "hello": "echo INSTALL_SMOKE_OK | tee hello.witness"
  }
}
EOF

echo "== herdr $(herdr --version) rustc $(rustc --version 2>/dev/null || echo absent) =="
if [ "$(herdr --version | awk '{print $2}')" != "0.9.1" ]; then
  echo "Herdr 0.9.1 is required" >&2
  exit 1
fi

herdr --session "$SESSION" server >"$DIAG/server.log" 2>&1 &
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

echo "== plugin install massdo/herdr-npm --ref $SHA =="
herdr --session "$SESSION" plugin install massdo/herdr-npm --ref "$SHA" --yes
if [ -f "$HERDR_NPM_BUILD_LOG" ]; then
  cat "$HERDR_NPM_BUILD_LOG"
fi
herdr --session "$SESSION" plugin list
herdr --session "$SESSION" config check

if [ "$PREBUILT" = 1 ]; then
  herdr --session "$SESSION" plugin list --plugin herdr-npm --json >"$DIAG/plugin.json"
  ROOT=$(python3 -c "import json,sys; print(json.load(sys.stdin)['result']['plugins'][0]['plugin_root'])" <"$DIAG/plugin.json")
  RESOLVED=$(python3 -c "import json,sys; print(json.load(sys.stdin)['result']['plugins'][0]['source']['resolved_commit'])" <"$DIAG/plugin.json")
  INSTALLED=$(git -C "$ROOT" rev-parse HEAD)
  VERSION=$(awk '/^\[/ { pkg = ($0 == "[package]") } pkg && /^version *= *"/ { sub(/^version *= *"/, ""); sub(/".*/, ""); print; exit }' "$ROOT/Cargo.toml")
  # The peeled line of an annotated tag comes last.
  TAG_COMMIT=$(git ls-remote https://github.com/massdo/herdr-npm "refs/tags/v$VERSION" "refs/tags/v$VERSION^{}" | awk '{ sha = $1 } END { print sha }')
  RELEASE="https://github.com/massdo/herdr-npm/releases/download/v$VERSION"
  curl -fsSL -o "$DIAG/SOURCE_COMMIT" "$RELEASE/SOURCE_COMMIT"
  curl -fsSL -o "$DIAG/SHA256SUMS" "$RELEASE/SHA256SUMS"
  SOURCE_COMMIT=$(cat "$DIAG/SOURCE_COMMIT")
  case "$(uname -s)/$(uname -m)" in
    Darwin/arm64) TRIPLE=aarch64-apple-darwin ;;
    Darwin/x86_64) TRIPLE=x86_64-apple-darwin ;;
    Linux/x86_64) TRIPLE=x86_64-unknown-linux-musl ;;
    *) echo "no prebuilt binary for this platform" >&2; exit 1 ;;
  esac
  PUBLISHED=$(awk -v name="herdr-npm-$TRIPLE" '$2 == name { print $1 }' "$DIAG/SHA256SUMS")
  if command -v sha256sum >/dev/null 2>&1; then
    SUM=$(sha256sum "$ROOT/target/release/herdr-npm")
  else
    SUM=$(shasum -a 256 "$ROOT/target/release/herdr-npm")
  fi
  echo "== commit: ref $SHA, installed $INSTALLED, resolved $RESOLVED, tag v$VERSION $TAG_COMMIT, SOURCE_COMMIT $SOURCE_COMMIT =="
  echo "== sha256: installed ${SUM%% *}, published $PUBLISHED =="
  for commit in "$INSTALLED" "$RESOLVED" "$TAG_COMMIT" "$SOURCE_COMMIT"; do
    if [ "$commit" != "$SHA" ]; then
      echo "commit mismatch: $commit is not $SHA" >&2
      exit 1
    fi
  done
  if [ -z "$PUBLISHED" ] || [ "${SUM%% *}" != "$PUBLISHED" ]; then
    echo "the installed binary is not the published herdr-npm-$TRIPLE" >&2
    exit 1
  fi
  if ! git -C "$ROOT" diff --quiet HEAD -- herdr-plugin.toml; then
    echo "herdr-plugin.toml changed during the install" >&2
    exit 1
  fi
  if ! grep -qF "installed verified prebuilt v$VERSION ($TRIPLE) for commit $SHA." "$HERDR_NPM_BUILD_LOG"; then
    echo "the build did not report the verified download" >&2
    exit 1
  fi
  if [ -e "$ROOT/target/release/deps" ]; then
    echo "target/release/deps exists: the plugin was compiled" >&2
    exit 1
  fi
fi

# Polls a condition every 0.2 s for at most $1 seconds.
wait_for() {
  tries=$(($1 * 5))
  shift
  while ! "$@"; do
    tries=$((tries - 1))
    [ "$tries" -gt 0 ] || return 1
    sleep 0.2
  done
}
sidebar_open() {
  herdr --session "$SESSION" pane list >"$DIAG/panes.json" && grep -q herdr_npm_sidebar "$DIAG/panes.json"
}
sidebar_closed() {
  herdr --session "$SESSION" pane list >"$DIAG/panes.json" && ! grep -q herdr_npm_sidebar "$DIAG/panes.json"
}
catalog_visible() {
  herdr --session "$SESSION" pane read "$PANE" --source visible --format text >"$DIAG/sidebar.txt" &&
    grep -q hello "$DIAG/sidebar.txt"
}
script_tab_open() {
  herdr --session "$SESSION" tab list >"$DIAG/tabs.json" && grep -q "npm run -- hello" "$DIAG/tabs.json"
}
witness_written() {
  grep -qx INSTALL_SMOKE_OK "$FIXTURE/hello.witness" 2>/dev/null
}

herdr --session "$SESSION" workspace create --cwd "$FIXTURE" --label install-smoke --no-focus >/dev/null
herdr --session "$SESSION" plugin action invoke herdr-npm.toggle
wait_for 10 sidebar_open || {
  echo "the sidebar did not open" >&2
  exit 1
}
PANE=$(python3 -c "import json,sys; panes=json.loads(sys.stdin.read())['result']['panes'];
print(next(p['pane_id'] for p in panes if (p.get('tokens') or {}).get('herdr_npm_sidebar')=='v1'))" <"$DIAG/panes.json")
wait_for 10 catalog_visible || {
  echo "the sidebar does not show the hello script" >&2
  exit 1
}
herdr --session "$SESSION" pane send-keys "$PANE" Enter
wait_for 10 script_tab_open || {
  echo "no tab runs npm run -- hello" >&2
  exit 1
}
wait_for 20 witness_written || {
  echo "hello did not write its witness file" >&2
  exit 1
}
herdr --session "$SESSION" pane send-keys "$PANE" q
wait_for 10 sidebar_closed || {
  echo "sidebar still present after q" >&2
  exit 1
}
if [ "$PREBUILT" = 1 ]; then MODE=prebuilt; else MODE=build; fi
echo "install_smoke_ok ref=$SHA session=$SESSION mode=$MODE"
